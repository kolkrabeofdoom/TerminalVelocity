<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * A leaf in the split tree. Each pane owns one terminal buffer and one
 * session, which is a simpler model than Rust's PaneRenderContext.
 */
final class Pane
{
    private TerminalBuffer $buffer;
    private AnsiParser $parser;
    private ?Session $session = null;
    private string $title = 'Shell';
    private string $workingDirectory = '';
    private bool $focused = false;
    private bool $activity = false;
    private int $createdAt;

    public function __construct(
        private int $id,
        private int $cols = 80,
        private int $rows = 24,
    ) {
        $this->buffer = new TerminalBuffer($cols, $rows);
        $this->parser = new AnsiParser($this->buffer);
        $this->createdAt = time();
    }

    public function id(): int
    {
        return $this->id;
    }

    public function buffer(): TerminalBuffer
    {
        return $this->buffer;
    }

    public function attach(Session $session): void
    {
        $this->session = $session;
        $this->title = $session->program();
        $this->workingDirectory = $session->workingDirectory();
    }

    public function session(): ?Session
    {
        return $this->session;
    }

    public function setFocus(bool $focused): void
    {
        $this->focused = $focused;
        if ($focused) {
            $this->activity = false;
        }
    }

    public function isFocused(): bool
    {
        return $this->focused;
    }

    public function markActivity(): void
    {
        if (!$this->focused) {
            $this->activity = true;
        }
    }

    public function hasActivity(): bool
    {
        return $this->activity;
    }

    public function setTitle(string $title): void
    {
        $this->title = $title;
    }

    public function title(): string
    {
        return $this->title;
    }

    public function workingDirectory(): string
    {
        return $this->workingDirectory;
    }

    public function write(string $data): void
    {
        $this->parser->feed($data);
        if (str_contains($data, "\x07")) {
            $this->markActivity();
        }
    }

    public function resize(int $cols, int $rows): void
    {
        $this->buffer->resize($cols, $rows);
    }

    public function restart(): void
    {
        if ($this->session === null) {
            return;
        }
        $program = $this->session->program();
        $directory = $this->session->workingDirectory();
        $this->session->terminate();
        $this->buffer->reset();
        $this->session = new Session($program, $directory);
        $this->session->start();
        $this->attach($this->session);
    }

    public function close(): void
    {
        if ($this->session !== null) {
            $this->session->terminate();
            $this->session = null;
        }
        $this->buffer->reset();
    }

    public function age(): int
    {
        return time() - $this->createdAt;
    }
}

/**
 * Binary split tree of panes. Supports nested horizontal and vertical splits
 * up to the documented limit of 64 panes.
 */
final class PaneTree
{
    private const MAX_PANES = 64;

    /** @var list<Pane|array{orientation:string,first:PaneTree,second:PaneTree,position:float}> */
    private array $nodes = [];

    private int $nextId = 1;
    private int $count = 0;

    public function __construct(private int $cols = 80, private int $rows = 24)
    {
        $this->nodes[] = $this->newPane();
        $this->count = 1;
    }

    private function newPane(): Pane
    {
        return new Pane($this->nextId++, $this->cols, $this->rows);
    }

    public function root(): Pane|array
    {
        return $this->nodes[0];
    }

    public function count(): int
    {
        return $this->count;
    }

    /** @return list<Pane> */
    public function leaves(): array
    {
        $result = [];
        $this->collectLeaves($this->nodes[0], $result);
        return $result;
    }

    /** @param list<Pane> $result */
    private function collectLeaves(Pane|array $node, array &$result): void
    {
        if ($node instanceof Pane) {
            $result[] = $node;
            return;
        }
        $this->collectLeaves($node['first'], $result);
        $this->collectLeaves($node['second'], $result);
    }

    public function split(string $orientation, Pane $target): Pane
    {
        if ($this->count >= self::MAX_PANES) {
            throw new \RuntimeException('Pane limit reached');
        }
        $newPane = $this->newPane();
        $this->replace($this->nodes[0], $target, [
            'orientation' => $orientation,
            'first' => $target,
            'second' => $newPane,
            'position' => 0.5,
        ]);
        $this->count++;
        return $newPane;
    }

    private function replace(Pane|array &$node, Pane $target, Pane|array $replacement): bool
    {
        if ($node instanceof Pane) {
            if ($node === $target) {
                $node = $replacement;
                return true;
            }
            return false;
        }
        if ($this->replace($node['first'], $target, $replacement)) {
            return true;
        }
        return $this->replace($node['second'], $target, $replacement);
    }

    public function resize(float $position, Pane|array &$node): void
    {
        if ($node instanceof Pane) {
            return;
        }
        $node['position'] = max(0.1, min(0.9, $position));
    }
}
