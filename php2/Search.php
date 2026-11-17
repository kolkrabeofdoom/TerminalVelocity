<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Literal, case-sensitive history search. The Rust version capped this at
 * 256 characters and 10000 matches; PHP 2.0 has no such limits.
 */
final class Search
{
    private string $query = '';
    private int $current = 0;

    /** @var list<int> */
    private array $matches = [];

    /** @param list<string> $lines */
    public function __construct(private array $lines)
    {
    }

    public function setQuery(string $query): void
    {
        $this->query = $query;
        $this->current = 0;
        $this->matches = [];
        foreach ($this->lines as $index => $line) {
            if (str_contains($line, $this->query)) {
                $this->matches[] = $index;
            }
        }
    }

    public function query(): string
    {
        return $this->query;
    }

    public function next(): ?int
    {
        if ($this->matches === []) {
            return null;
        }
        $result = $this->matches[$this->current % count($this->matches)];
        $this->current++;
        return $result;
    }

    public function previous(): ?int
    {
        if ($this->matches === []) {
            return null;
        }
        $this->current = $this->current === 0 ? count($this->matches) - 1 : $this->current - 1;
        return $this->matches[$this->current];
    }

    public function close(): void
    {
        $this->query = '';
        $this->matches = [];
        $this->current = 0;
    }

    public function count(): int
    {
        return count($this->matches);
    }

    /** @return list<int> */
    public function matches(): array
    {
        return $this->matches;
    }
}
