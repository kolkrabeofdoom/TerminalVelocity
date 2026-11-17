<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Application orchestration. Combines the split tree, sessions, editor,
 * search, palette and renderer into the main event loop.
 */
final class App
{
    private const MAX_TABS = 32;

    private PaneTree $tree;
    private ?Pane $activePane = null;
    private ?Editor $editor = null;
    private ?Search $search = null;
    private bool $running = false;

    public function __construct(
        private Config $config,
        private Renderer $renderer,
    ) {
        $this->tree = new PaneTree();
        $this->activePane = $this->tree->leaves()[0] ?? null;
    }

    public function start(): void
    {
        $this->running = true;
        foreach ($this->config->tabs() as $tab) {
            $program = (string) ($tab['program'] ?? 'pwsh');
            $directory = (string) ($tab['directory'] ?? '');
            $this->newTab($program, $directory);
        }
    }

    public function newTab(string $program = 'pwsh', string $directory = ''): Pane
    {
        if (count($this->tree->leaves()) >= self::MAX_TABS) {
            throw new \RuntimeException('Tab limit reached');
        }
        $pane = $this->tree->leaves()[0] ?? null;
        if ($this->activePane !== null && $this->activePane->session() === null) {
            $pane = $this->activePane;
        }
        $pane ??= $this->tree->leaves()[0];
        $session = new Session($program, $directory);
        $session->start();
        $pane->attach($session);
        $this->setActivePane($pane);
        return $pane;
    }

    public function split(string $orientation): ?Pane
    {
        if ($this->activePane === null) {
            return null;
        }
        $pane = $this->tree->split($orientation, $this->activePane);
        $session = new Session('pwsh', $this->activePane->workingDirectory());
        $session->start();
        $pane->attach($session);
        $pane->setFocus(true);
        $this->activePane->setFocus(false);
        $this->activePane = $pane;
        return $pane;
    }

    private function setActivePane(Pane $pane): void
    {
        foreach ($this->tree->leaves() as $leaf) {
            $leaf->setFocus($leaf === $pane);
        }
        $this->activePane = $pane;
    }

    public function activePane(): ?Pane
    {
        return $this->activePane;
    }

    public function write(string $data): void
    {
        if ($this->activePane !== null) {
            $this->activePane->write($data);
        }
    }

    public function restartSession(): void
    {
        if ($this->activePane !== null) {
            $this->activePane->restart();
        }
    }

    public function closePane(): void
    {
        if ($this->activePane === null) {
            return;
        }
        $this->activePane->close();
        $leaves = $this->tree->leaves();
        $this->activePane = $leaves[0] ?? null;
    }

    public function openEditor(): Editor
    {
        $this->editor ??= new Editor();
        return $this->editor;
    }

    public function editor(): ?Editor
    {
        return $this->editor;
    }

    public function beginSearch(string $query): void
    {
        $lines = $this->activePane?->buffer()->history() ?? [];
        $this->search = new Search($lines);
        $this->search->setQuery($query);
    }

    public function search(): ?Search
    {
        return $this->search;
    }

    public function endSearch(): void
    {
        $this->search?->close();
        $this->search = null;
    }

    public function setPreset(string $preset): bool
    {
        $result = $this->renderer->setPreset($preset);
        if ($result) {
            $this->config->setTheme($preset);
        }
        return $result;
    }

    public function save(): bool
    {
        $tabs = [];
        foreach ($this->tree->leaves() as $pane) {
            $tabs[] = [
                'program' => $pane->title(),
                'directory' => $pane->workingDirectory(),
            ];
        }
        $this->config->setTabs($tabs);
        $this->config->setTheme($this->renderer->preset());
        return $this->config->save();
    }

    public function render(): string
    {
        if ($this->activePane === null) {
            return '';
        }
        return $this->renderer->render($this->activePane->buffer());
    }

    public function pollSessions(): void
    {
        foreach ($this->tree->leaves() as $pane) {
            $session = $pane->session();
            if ($session === null) {
                continue;
            }
            $output = $session->read();
            if ($output !== '') {
                $pane->write($output);
            }
            if (!$session->poll() && !$pane->isFocused()) {
                $pane->markActivity();
            }
        }
    }

    public function eventLoop(int $iterations = 1): int
    {
        for ($i = 0; $i < $iterations && $this->running; $i++) {
            $this->pollSessions();
        }
        return $this->activePane === null ? 1 : 0;
    }

    public function stop(): void
    {
        foreach ($this->tree->leaves() as $pane) {
            $pane->close();
        }
        $this->running = false;
    }
}
