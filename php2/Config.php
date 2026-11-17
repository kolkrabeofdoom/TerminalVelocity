<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Workspace and project profiles. Replaces workspace.json/profiles.json
 * handling from src/model.rs and src/profiles.rs.
 */
final class Config
{
    private const MAX_PROFILES = 32;
    private const MAX_CONNECTIONS = 128;

    /** @var array<string, mixed> */
    private array $workspace = [];

    /** @var array<string, array<string, mixed>> */
    private array $profiles = [];

    public function __construct(
        private string $directory = '.',
    ) {
        $this->workspace = [
            'tabs' => [],
            'panes' => [],
            'workingDirectory' => '',
            'theme' => 'Aurora',
            'intensity' => 0.45,
            'animations' => true,
            'backgroundImage' => '',
        ];
    }

    public function load(): bool
    {
        $workspacePath = $this->directory . '/workspace.json';
        if (is_file($workspacePath)) {
            $data = json_decode((string) file_get_contents($workspacePath), true);
            if (is_array($data)) {
                $this->workspace = array_merge($this->workspace, $data);
            }
        }
        $profilesPath = $this->directory . '/profiles.json';
        if (is_file($profilesPath)) {
            $data = json_decode((string) file_get_contents($profilesPath), true);
            if (is_array($data)) {
                $this->profiles = $data;
            }
        }
        return true;
    }

    public function save(): bool
    {
        return $this->writeJson($this->directory . '/workspace.json', $this->workspace)
            && $this->writeJson($this->directory . '/profiles.json', $this->profiles);
    }

    /** @param array<string, mixed> $data */
    private function writeJson(string $path, array $data): bool
    {
        $temporary = $path . '.tmp-' . getmypid();
        $json = json_encode($data, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES);
        if ($json === false) {
            return false;
        }
        if (file_put_contents($temporary, $json, LOCK_EX) === false) {
            return false;
        }
        if (!rename($temporary, $path)) {
            unlink($temporary);
            return false;
        }
        return true;
    }

    /** @param list<array<string, mixed>> $tabs */
    public function setTabs(array $tabs): void
    {
        $this->workspace['tabs'] = array_slice($tabs, 0, 32);
    }

    /** @return list<array<string, mixed>> */
    public function tabs(): array
    {
        return $this->workspace['tabs'] ?? [];
    }

    public function setTheme(string $theme): void
    {
        $allowed = ['Off', 'Aurora', 'Phosphor', 'Deep Space'];
        if (in_array($theme, $allowed, true)) {
            $this->workspace['theme'] = $theme;
        }
    }

    public function theme(): string
    {
        return $this->workspace['theme'] ?? 'Aurora';
    }

    public function setIntensity(float $intensity): void
    {
        $this->workspace['intensity'] = max(0.0, min(1.0, $intensity));
    }

    public function intensity(): float
    {
        return (float) ($this->workspace['intensity'] ?? 0.45);
    }

    public function saveProfile(string $name, array $layout): bool
    {
        if ($name === '' || mb_strlen($name) > 80) {
            return false;
        }
        if (!isset($this->profiles[$name]) && count($this->profiles) >= self::MAX_PROFILES) {
            return false;
        }
        $this->profiles[$name] = $layout;
        return true;
    }

    public function deleteProfile(string $name): bool
    {
        if (!isset($this->profiles[$name])) {
            return false;
        }
        unset($this->profiles[$name]);
        return true;
    }

    public function profile(string $name): ?array
    {
        return $this->profiles[$name] ?? null;
    }

    /** @return list<string> */
    public function profileNames(): array
    {
        return array_keys($this->profiles);
    }

    public function addConnection(array $connection): bool
    {
        $connections = $this->workspace['connections'] ?? [];
        if (count($connections) >= self::MAX_CONNECTIONS) {
            return false;
        }
        $connections[] = $connection;
        $this->workspace['connections'] = $connections;
        return true;
    }

    /** @return list<array<string, mixed>> */
    public function connections(): array
    {
        return $this->workspace['connections'] ?? [];
    }

    public function setBackgroundImage(string $path): void
    {
        $this->workspace['backgroundImage'] = $path;
    }

    public function backgroundImage(): string
    {
        return $this->workspace['backgroundImage'] ?? '';
    }

    public function directory(): string
    {
        return $this->directory;
    }
}
