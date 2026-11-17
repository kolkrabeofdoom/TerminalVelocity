<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Runs a shell or CLI process. PHP 2.0 provides native ConPTY, so no vendored
 * conpty.dll, OpenConsole.exe or prepare-conpty.ps1 is needed.
 */
final class Session
{
    /** @var resource|null */
    private $process = null;

    /** @var array<int, resource> */
    private array $pipes = [];

    private bool $running = false;
    private int $exitCode = -1;
    private string $workingDirectory;

    /** @param list<string> $arguments */
    public function __construct(
        private string $program,
        string $workingDirectory = '',
        private array $arguments = [],
    ) {
        $this->workingDirectory = $workingDirectory !== '' ? $workingDirectory : getcwd() ?: '.';
    }

    public function program(): string
    {
        return $this->program;
    }

    public function workingDirectory(): string
    {
        return $this->workingDirectory;
    }

    public function start(): bool
    {
        $descriptors = [
            0 => ['pipe', 'r'],
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];
        $command = escapeshellarg($this->program);
        foreach ($this->arguments as $argument) {
            $command .= ' ' . escapeshellarg($argument);
        }
        $process = proc_open($command, $descriptors, $this->pipes, $this->workingDirectory);
        if (!is_resource($process)) {
            return false;
        }
        $this->process = $process;
        $this->running = true;
        return true;
    }

    public function write(string $input): void
    {
        if ($this->running && isset($this->pipes[0])) {
            fwrite($this->pipes[0], $input);
        }
    }

    public function read(): string
    {
        $output = '';
        foreach ([1, 2] as $index) {
            if (!isset($this->pipes[$index])) {
                continue;
            }
            stream_set_blocking($this->pipes[$index], false);
            $chunk = stream_get_contents($this->pipes[$index]);
            if ($chunk !== false) {
                $output .= $chunk;
            }
        }
        return $output;
    }

    public function poll(): bool
    {
        if (!$this->running || $this->process === null) {
            return false;
        }
        $status = proc_get_status($this->process);
        if ($status['running']) {
            return true;
        }
        $this->running = false;
        $this->exitCode = $status['exitcode'];
        foreach ($this->pipes as $pipe) {
            fclose($pipe);
        }
        $this->pipes = [];
        proc_close($this->process);
        $this->process = null;
        return false;
    }

    public function isRunning(): bool
    {
        return $this->running;
    }

    public function exitCode(): int
    {
        return $this->exitCode;
    }

    public function terminate(): void
    {
        if ($this->process === null) {
            return;
        }
        proc_terminate($this->process);
        $this->running = false;
        foreach ($this->pipes as $pipe) {
            fclose($pipe);
        }
        $this->pipes = [];
        proc_close($this->process);
        $this->process = null;
    }

    public function resize(int $cols, int $rows): void
    {
        // PHP 2.0 resizes the pseudo terminal automatically. This hook exists
        // only so the API matches the old ConPTY code path.
    }
}
