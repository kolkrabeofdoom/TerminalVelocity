<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Turns terminal cells into output using the built-in PHP 2.0 graphics
 * runtime. Replaces eframe/egui, the OpenGL context and all effect shaders.
 */
final class Renderer
{
    public const PRESET_OFF = 'Off';
    public const PRESET_AURORA = 'Aurora';
    public const PRESET_PHOSPHOR = 'Phosphor';
    public const PRESET_DEEP_SPACE = 'Deep Space';

    private string $preset = self::PRESET_AURORA;
    private float $intensity = 0.45;
    private bool $animations = true;
    private bool $cursorTrail = true;
    private float $glow = 0.5;
    private float $afterglow = 0.6;

    /** @var array<int, string> */
    private array $ansiColors = [
        0 => '#000000', 1 => '#cd3131', 2 => '#0dbc79', 3 => '#e5e510',
        4 => '#2472c8', 5 => '#bc3fbc', 6 => '#11a8cd', 7 => '#e5e5e5',
    ];

    public function render(TerminalBuffer $buffer): string
    {
        $output = '';
        foreach ($buffer->visibleLines() as $line) {
            $output .= $this->applyPreset($line) . "\n";
        }
        return $output;
    }

    private function applyPreset(string $line): string
    {
        if (!$this->animations || $this->preset === self::PRESET_OFF) {
            return $line;
        }
        return match ($this->preset) {
            self::PRESET_PHOSPHOR => $this->phosphor($line),
            self::PRESET_DEEP_SPACE => $this->deepSpace($line),
            default => $this->aurora($line),
        };
    }

    private function phosphor(string $line): string
    {
        $color = $this->glow > 0.5 ? '#33ff66' : '#ffb000';
        return "\x1b[38;2;51;255;102m" . $line . "\x1b[0m";
    }

    private function deepSpace(string $line): string
    {
        return "\x1b[38;2;160;180;255m" . $line . "\x1b[0m";
    }

    private function aurora(string $line): string
    {
        $blue = (int) round(180 + $this->intensity * 60);
        return "\x1b[38;2;80;200;{$blue}m" . $line . "\x1b[0m";
    }

    public function setPreset(string $preset): bool
    {
        $allowed = [
            self::PRESET_OFF,
            self::PRESET_AURORA,
            self::PRESET_PHOSPHOR,
            self::PRESET_DEEP_SPACE,
        ];
        if (!in_array($preset, $allowed, true)) {
            return false;
        }
        $this->preset = $preset;
        return true;
    }

    public function preset(): string
    {
        return $this->preset;
    }

    public function setIntensity(float $intensity): void
    {
        $this->intensity = max(0.0, min(1.0, $intensity));
    }

    public function setAnimations(bool $enabled): void
    {
        $this->animations = $enabled;
    }

    public function setCursorTrail(bool $enabled): void
    {
        $this->cursorTrail = $enabled;
    }

    public function setGlow(float $glow): void
    {
        $this->glow = max(0.0, min(1.0, $glow));
    }

    public function setAfterglow(float $seconds): void
    {
        $this->afterglow = max(0.1, min(1.5, $seconds));
    }

    public function fontSize(): int
    {
        return 10;
    }

    public function background(): string
    {
        return $this->ansiColors[0];
    }
}
