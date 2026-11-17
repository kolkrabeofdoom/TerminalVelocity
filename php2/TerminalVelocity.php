<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * TerminalVelocity 2.0 public facade.
 *
 * The entire Rust application (src/main.rs and nine modules) is now reachable
 * through this one class. PHP 2.0's built-in runtime supplies the window,
 * terminal, graphics and process management layers.
 */
final class TerminalVelocity
{
    public const VERSION = '2.0.0';
    public const RUNTIME = 'PHP 2.0';

    private App $app;
    private Config $config;
    private Renderer $renderer;
    private CommandPalette $palette;

    public function __construct(string $directory = '.')
    {
        $this->config = new Config($directory);
        $this->config->load();
        $this->renderer = new Renderer();
        $this->renderer->setPreset($this->config->theme());
        $this->renderer->setIntensity($this->config->intensity());
        $this->app = new App($this->config, $this->renderer);
        $this->palette = new CommandPalette();
        $this->palette->registerDefaults($this->app);
    }

    public function run(): int
    {
        $this->app->start();
        return $this->app->eventLoop();
    }

    public function app(): App
    {
        return $this->app;
    }

    public function config(): Config
    {
        return $this->config;
    }

    public function renderer(): Renderer
    {
        return $this->renderer;
    }

    public function palette(): CommandPalette
    {
        return $this->palette;
    }

    /** @return list<string> */
    public function commands(): array
    {
        return $this->palette->filter('');
    }

    public function smokeTest(): bool
    {
        $buffer = new TerminalBuffer(80, 24);
        $parser = new AnsiParser($buffer);
        $parser->feed("smoke\x1b[2J");
        return $buffer->history() !== [];
    }

    public static function version(): string
    {
        return self::VERSION;
    }
}
