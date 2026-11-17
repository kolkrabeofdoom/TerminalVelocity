<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Local text editor. Supports UTF-8, LF/CRLF, undo/redo and atomic saves,
 * all of which were previously spread over src/editor.rs.
 */
final class Editor
{
    private const MAX_BYTES = 256 * 1024;

    /** @var list<string> */
    private array $lines = [''];

    /** @var list<list<string>> */
    private array $undoStack = [];

    /** @var list<list<string>> */
    private array $redoStack = [];

    private string $path = '';
    private string $encoding = 'UTF-8';
    private string $lineEnding = "\n";
    private bool $dirty = false;
    private int $cursorLine = 0;
    private int $cursorColumn = 0;

    public function open(string $path): bool
    {
        if (!is_file($path)) {
            return false;
        }
        $contents = file_get_contents($path);
        if ($contents === false || strlen($contents) > self::MAX_BYTES) {
            return false;
        }
        if (!mb_check_encoding($contents, 'UTF-8')) {
            return false;
        }
        $this->path = $path;
        $this->lineEnding = str_contains($contents, "\r\n") ? "\r\n" : "\n";
        $this->encoding = str_starts_with($contents, "\xef\xbb\xbf") ? 'UTF-8 BOM' : 'UTF-8';
        $normalized = str_replace("\r\n", "\n", $contents);
        $this->lines = explode("\n", $normalized);
        $this->dirty = false;
        $this->undoStack = [];
        $this->redoStack = [];
        return true;
    }

    public function newDocument(): void
    {
        $this->path = '';
        $this->lines = [''];
        $this->encoding = 'UTF-8';
        $this->lineEnding = "\n";
        $this->dirty = false;
        $this->undoStack = [];
        $this->redoStack = [];
    }

    public function insert(string $text): void
    {
        $this->snapshot();
        $line = $this->lines[$this->cursorLine] ?? '';
        $before = mb_substr($line, 0, $this->cursorColumn);
        $after = mb_substr($line, $this->cursorColumn);
        $this->lines[$this->cursorLine] = $before . $text . $after;
        $this->cursorColumn += mb_strlen($text);
        $this->dirty = true;
    }

    public function deleteBefore(): void
    {
        if ($this->cursorColumn === 0 && $this->cursorLine === 0) {
            return;
        }
        $this->snapshot();
        if ($this->cursorColumn === 0) {
            $previous = $this->lines[$this->cursorLine - 1] ?? '';
            $current = $this->lines[$this->cursorLine] ?? '';
            $this->cursorColumn = mb_strlen($previous);
            array_splice($this->lines, $this->cursorLine - 1, 2, [$previous . $current]);
            $this->cursorLine--;
        } else {
            $line = $this->lines[$this->cursorLine];
            $this->lines[$this->cursorLine] = mb_substr($line, 0, $this->cursorColumn - 1)
                . mb_substr($line, $this->cursorColumn);
            $this->cursorColumn--;
        }
        $this->dirty = true;
    }

    public function newline(): void
    {
        $this->snapshot();
        $line = $this->lines[$this->cursorLine] ?? '';
        $before = mb_substr($line, 0, $this->cursorColumn);
        $after = mb_substr($line, $this->cursorColumn);
        $this->lines[$this->cursorLine] = $before;
        array_splice($this->lines, $this->cursorLine + 1, 0, [$after]);
        $this->cursorLine++;
        $this->cursorColumn = 0;
        $this->dirty = true;
    }

    private function snapshot(): void
    {
        $this->undoStack[] = $this->lines;
        if (count($this->undoStack) > 200) {
            array_shift($this->undoStack);
        }
        $this->redoStack = [];
    }

    public function undo(): bool
    {
        if ($this->undoStack === []) {
            return false;
        }
        $this->redoStack[] = $this->lines;
        $this->lines = array_pop($this->undoStack);
        $this->dirty = true;
        return true;
    }

    public function redo(): bool
    {
        if ($this->redoStack === []) {
            return false;
        }
        $this->undoStack[] = $this->lines;
        $this->lines = array_pop($this->redoStack);
        $this->dirty = true;
        return true;
    }

    public function setCursor(int $line, int $column): void
    {
        $this->cursorLine = max(0, min($line, count($this->lines) - 1));
        $length = mb_strlen($this->lines[$this->cursorLine] ?? '');
        $this->cursorColumn = max(0, min($column, $length));
    }

    /** @return array{int,int} */
    public function cursor(): array
    {
        return [$this->cursorLine, $this->cursorColumn];
    }

    /** @return list<string> */
    public function lines(): array
    {
        return $this->lines;
    }

    public function contents(): string
    {
        $text = implode($this->lineEnding, $this->lines);
        if ($this->encoding === 'UTF-8 BOM') {
            $text = "\xef\xbb\xbf" . $text;
        }
        return $text;
    }

    public function save(string $path = ''): bool
    {
        $target = $path !== '' ? $path : $this->path;
        if ($target === '') {
            return false;
        }
        $contents = $this->contents();
        if (strlen($contents) > self::MAX_BYTES) {
            return false;
        }
        $temporary = $target . '.tmp-' . getmypid();
        if (file_put_contents($temporary, $contents, LOCK_EX) === false) {
            return false;
        }
        if (!rename($temporary, $target)) {
            unlink($temporary);
            return false;
        }
        $this->path = $target;
        $this->dirty = false;
        return true;
    }

    /** @return list<int> */
    public function find(string $needle): array
    {
        $matches = [];
        foreach ($this->lines as $index => $line) {
            if (str_contains($line, $needle)) {
                $matches[] = $index;
            }
        }
        return $matches;
    }

    public function path(): string
    {
        return $this->path;
    }

    public function isDirty(): bool
    {
        return $this->dirty;
    }

    public function encoding(): string
    {
        return $this->encoding;
    }

    public function lineEnding(): string
    {
        return $this->lineEnding === "\n" ? 'LF' : 'CRLF';
    }
}
