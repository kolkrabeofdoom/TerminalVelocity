<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * One character cell in the terminal grid. PHP 2.0 stores cells by value,
 * which removes the need for the vendored Rust grid implementation.
 */
final class Cell
{
    public const FLAG_BOLD = 1;
    public const FLAG_DIM = 2;
    public const FLAG_ITALIC = 4;
    public const FLAG_UNDERLINE = 8;
    public const FLAG_BLINK = 16;
    public const FLAG_REVERSE = 32;
    public const FLAG_HIDDEN = 64;
    public const FLAG_STRIKETHROUGH = 128;

    public function __construct(
        public string $char = ' ',
        public int $foreground = 0,
        public int $background = 0,
        public int $flags = 0,
    ) {
    }

    public function isBlank(): bool
    {
        return $this->char === ' ' && $this->flags === 0;
    }

    public function equals(self $other): bool
    {
        return $this->char === $other->char
            && $this->foreground === $other->foreground
            && $this->background === $other->background
            && $this->flags === $other->flags;
    }

    public function withChar(string $char): self
    {
        return new self($char, $this->foreground, $this->background, $this->flags);
    }

    public function __toString(): string
    {
        return $this->char;
    }
}

/**
 * The visible grid plus bounded scrollback. Replaces egui_term and the
 * patched alacritty_terminal grid modules.
 */
final class TerminalBuffer
{
    /** @var list<list<Cell>> */
    private array $rows = [];

    /** @var list<list<Cell>> */
    private array $scrollback = [];

    private int $cursorX = 0;
    private int $cursorY = 0;
    private int $savedCursorX = 0;
    private int $savedCursorY = 0;
    private int $scrollTop = 0;
    private int $scrollBottom = 0;

    public function __construct(
        private int $cols = 80,
        private int $rows = 24,
        private int $scrollbackLimit = 10000,
    ) {
        $this->reset();
    }

    public function reset(): void
    {
        $this->rows = [];
        $this->scrollback = [];
        $this->cursorX = 0;
        $this->cursorY = 0;
        $this->scrollTop = 0;
        $this->scrollBottom = $this->rows - 1;
        for ($y = 0; $y < $this->rows; $y++) {
            $this->rows[$y] = $this->blankRow();
        }
    }

    /** @return list<Cell> */
    private function blankRow(): array
    {
        $row = [];
        for ($x = 0; $x < $this->cols; $x++) {
            $row[] = new Cell();
        }
        return $row;
    }

    public function resize(int $cols, int $rows): void
    {
        if ($cols < 1 || $rows < 1) {
            throw new \InvalidArgumentException('Grid dimensions must be positive');
        }
        $this->cols = $cols;
        $this->rows = $rows;
        $this->scrollBottom = $rows - 1;
        foreach ($this->rows as $y => $row) {
            $this->rows[$y] = array_slice($row, 0, $cols);
            while (count($this->rows[$y]) < $cols) {
                $this->rows[$y][] = new Cell();
            }
        }
        while (count($this->rows) < $rows) {
            $this->rows[] = $this->blankRow();
        }
        $this->rows = array_slice($this->rows, 0, $rows);
        $this->cursorX = min($this->cursorX, $cols - 1);
        $this->cursorY = min($this->cursorY, $rows - 1);
    }

    public function put(string $char, int $foreground, int $background, int $flags): void
    {
        if ($this->cursorX >= $this->cols) {
            $this->lineFeed();
        }
        $this->rows[$this->cursorY][$this->cursorX] = new Cell($char, $foreground, $background, $flags);
        $this->cursorX++;
    }

    public function lineFeed(): void
    {
        if ($this->cursorY === $this->scrollBottom) {
            $this->scrollUp(1);
            return;
        }
        $this->cursorY++;
    }

    public function scrollUp(int $count): void
    {
        for ($i = 0; $i < $count; $i++) {
            $this->scrollback[] = array_shift($this->rows);
            $this->rows[] = $this->blankRow();
        }
        while (count($this->scrollback) > $this->scrollbackLimit) {
            array_shift($this->scrollback);
        }
    }

    public function carriageReturn(): void
    {
        $this->cursorX = 0;
    }

    public function backspace(): void
    {
        if ($this->cursorX > 0) {
            $this->cursorX--;
        }
    }

    public function setCursor(int $x, int $y): void
    {
        $this->cursorX = max(0, min($x, $this->cols - 1));
        $this->cursorY = max(0, min($y, $this->rows - 1));
    }

    public function saveCursor(): void
    {
        $this->savedCursorX = $this->cursorX;
        $this->savedCursorY = $this->cursorY;
    }

    public function restoreCursor(): void
    {
        $this->setCursor($this->savedCursorX, $this->savedCursorY);
    }

    public function eraseInLine(int $mode): void
    {
        $row = &$this->rows[$this->cursorY];
        if ($mode === 0) {
            for ($x = $this->cursorX; $x < $this->cols; $x++) {
                $row[$x] = new Cell();
            }
        } elseif ($mode === 1) {
            for ($x = 0; $x <= $this->cursorX; $x++) {
                $row[$x] = new Cell();
            }
        } elseif ($mode === 2) {
            $this->rows[$this->cursorY] = $this->blankRow();
        }
    }

    public function eraseInDisplay(int $mode): void
    {
        if ($mode === 0) {
            $this->eraseInLine(0);
            for ($y = $this->cursorY + 1; $y < $this->rows; $y++) {
                $this->rows[$y] = $this->blankRow();
            }
        } elseif ($mode === 1) {
            $this->eraseInLine(1);
            for ($y = 0; $y < $this->cursorY; $y++) {
                $this->rows[$y] = $this->blankRow();
            }
        } elseif ($mode === 2 || $mode === 3) {
            $this->reset();
        }
    }

    public function setScrollRegion(int $top, int $bottom): void
    {
        if ($top < 0 || $bottom >= $this->rows || $top >= $bottom) {
            return;
        }
        $this->scrollTop = $top;
        $this->scrollBottom = $bottom;
        $this->setCursor(0, 0);
    }

    /** @return list<string> */
    public function visibleLines(): array
    {
        $lines = [];
        foreach ($this->rows as $row) {
            $text = '';
            foreach ($row as $cell) {
                $text .= $cell->char;
            }
            $lines[] = rtrim($text);
        }
        return $lines;
    }

    /** @return list<string> */
    public function history(): array
    {
        $lines = [];
        foreach ($this->scrollback as $row) {
            $text = '';
            foreach ($row as $cell) {
                $text .= $cell->char;
            }
            $lines[] = rtrim($text);
        }
        return array_merge($lines, $this->visibleLines());
    }

    public function cursor(): array
    {
        return [$this->cursorX, $this->cursorY];
    }

    public function dimensions(): array
    {
        return [$this->cols, $this->rows];
    }
}
