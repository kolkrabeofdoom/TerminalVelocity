<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Keyword-filtered command palette. Replaces the actions from src/main.rs.
 */
final class CommandPalette
{
    /** @var array<string, callable> */
    private array $commands = [];

    public function register(string $name, callable $handler): void
    {
        $this->commands[$name] = $handler;
    }

    /** @return list<string> */
    public function filter(string $query): array
    {
        $query = mb_strtolower(trim($query));
        $results = [];
        foreach (array_keys($this->commands) as $name) {
            if ($query === '' || str_contains(mb_strtolower($name), $query)) {
                $results[] = $name;
            }
        }
        return $results;
    }

    public function execute(string $name): bool
    {
        if (!isset($this->commands[$name])) {
            return false;
        }
        ($this->commands[$name])();
        return true;
    }

    public function has(string $name): bool
    {
        return isset($this->commands[$name]);
    }

    public function count(): int
    {
        return count($this->commands);
    }

    public function registerDefaults(App $app): void
    {
        $this->register('New tab', fn () => $app->newTab());
        $this->register('Split right', fn () => $app->split('horizontal'));
        $this->register('Split down', fn () => $app->split('vertical'));
        $this->register('Save workspace', fn () => $app->save());
        $this->register('Open editor', fn () => $app->openEditor());
        $this->register('Toggle Aurora', fn () => $app->setPreset('Aurora'));
        $this->register('Toggle Phosphor', fn () => $app->setPreset('Phosphor'));
        $this->register('Toggle Deep Space', fn () => $app->setPreset('Deep Space'));
        $this->register('Restart session', fn () => $app->restartSession());
    }
}
