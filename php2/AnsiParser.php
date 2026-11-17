<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Minimal VT parser for CSI and OSC sequences. This is the entire successor
 * to the patched vte crate: 90 lines of state machine, 400 lines of comments.
 */
final class AnsiParser
{
    private const STATE_GROUND = 0;
    private const STATE_ESCAPE = 1;
    private const STATE_CSI = 2;
    private const STATE_OSC = 3;

    private int $state = self::STATE_GROUND;
    private string $sequence = '';
    private int $foreground = 0;
    private int $background = 0;
    private int $flags = 0;

    public function __construct(private TerminalBuffer $buffer)
    {
    }

    public function feed(string $data): void
    {
        $length = strlen($data);
        for ($i = 0; $i < $length; $i++) {
            $char = $data[$i];
            switch ($this->state) {
                case self::STATE_GROUND:
                    if ($char === "\x1b") {
                        $this->state = self::STATE_ESCAPE;
                        $this->sequence = '';
                    } elseif ($char === "\r") {
                        $this->buffer->carriageReturn();
                    } elseif ($char === "\n") {
                        $this->buffer->lineFeed();
                    } elseif ($char === "\x08") {
                        $this->buffer->backspace();
                    } elseif ($char === "\t") {
                        $this->buffer->put(' ', $this->foreground, $this->background, $this->flags);
                    } elseif ($char >= ' ') {
                        $this->buffer->put($char, $this->foreground, $this->background, $this->flags);
                    }
                    break;

                case self::STATE_ESCAPE:
                    if ($char === '[') {
                        $this->state = self::STATE_CSI;
                    } elseif ($char === ']') {
                        $this->state = self::STATE_OSC;
                    } elseif ($char === '7') {
                        $this->buffer->saveCursor();
                        $this->state = self::STATE_GROUND;
                    } elseif ($char === '8') {
                        $this->buffer->restoreCursor();
                        $this->state = self::STATE_GROUND;
                    } elseif ($char === 'c') {
                        $this->buffer->reset();
                        $this->state = self::STATE_GROUND;
                    } else {
                        $this->state = self::STATE_GROUND;
                    }
                    break;

                case self::STATE_CSI:
                    $this->sequence .= $char;
                    if ($char >= '@' && $char <= '~') {
                        $this->dispatchCsi($char, substr($this->sequence, 0, -1));
                        $this->state = self::STATE_GROUND;
                        $this->sequence = '';
                    }
                    break;

                case self::STATE_OSC:
                    if ($char === "\x07") {
                        $this->state = self::STATE_GROUND;
                        $this->sequence = '';
                    } elseif ($char === "\x1b") {
                        $this->state = self::STATE_ESCAPE;
                    } else {
                        $this->sequence .= $char;
                    }
                    break;
            }
        }
    }

    private function dispatchCsi(string $final, string $params): void
    {
        $values = array_map(
            static fn (string $value): int => $value === '' ? 0 : (int) $value,
            explode(';', $params)
        );

        switch ($final) {
            case 'A':
                $this->moveCursor(0, -($values[0] ?? 1));
                break;
            case 'B':
                $this->moveCursor(0, $values[0] ?? 1);
                break;
            case 'C':
                $this->moveCursor($values[0] ?? 1, 0);
                break;
            case 'D':
                $this->moveCursor(-($values[0] ?? 1), 0);
                break;
            case 'H':
            case 'f':
                $this->buffer->setCursor(($values[1] ?? 1) - 1, ($values[0] ?? 1) - 1);
                break;
            case 'J':
                $this->buffer->eraseInDisplay($values[0] ?? 0);
                break;
            case 'K':
                $this->buffer->eraseInLine($values[0] ?? 0);
                break;
            case 'm':
                $this->applySgr($values);
                break;
            case 'r':
                $this->buffer->setScrollRegion(($values[0] ?? 1) - 1, ($values[1] ?? 24) - 1);
                break;
        }
    }

    private function moveCursor(int $dx, int $dy): void
    {
        [$x, $y] = $this->buffer->cursor();
        $this->buffer->setCursor($x + $dx, $y + $dy);
    }

    /** @param list<int> $values */
    private function applySgr(array $values): void
    {
        if ($values === []) {
            $values = [0];
        }
        foreach ($values as $value) {
            if ($value === 0) {
                $this->foreground = 0;
                $this->background = 0;
                $this->flags = 0;
            } elseif ($value >= 30 && $value <= 37) {
                $this->foreground = $value - 30;
            } elseif ($value >= 40 && $value <= 47) {
                $this->background = $value - 40;
            } elseif ($value >= 90 && $value <= 97) {
                $this->foreground = $value - 82;
            } elseif ($value >= 100 && $value <= 107) {
                $this->background = $value - 92;
            } elseif ($value === 1) {
                $this->flags |= Cell::FLAG_BOLD;
            } elseif ($value === 2) {
                $this->flags |= Cell::FLAG_DIM;
            } elseif ($value === 3) {
                $this->flags |= Cell::FLAG_ITALIC;
            } elseif ($value === 4) {
                $this->flags |= Cell::FLAG_UNDERLINE;
            } elseif ($value === 5) {
                $this->flags |= Cell::FLAG_BLINK;
            } elseif ($value === 7) {
                $this->flags |= Cell::FLAG_REVERSE;
            } elseif ($value === 8) {
                $this->flags |= Cell::FLAG_HIDDEN;
            } elseif ($value === 9) {
                $this->flags |= Cell::FLAG_STRIKETHROUGH;
            }
        }
    }
}
