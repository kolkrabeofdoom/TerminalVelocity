<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * SSH and WinRM connections. The Rust version launched ssh.exe and
 * Enter-PSSession; PHP 2.0 connects directly from the runtime.
 */
final class Remote
{
    public const TYPE_SSH = 'ssh';
    public const TYPE_WINRM = 'winrm';

    public function __construct(
        private string $name,
        private string $host,
        private string $type = self::TYPE_SSH,
        private string $account = '',
        private int $port = 0,
    ) {
    }

    /** @param array<string, mixed> $data */
    public static function fromArray(array $data): self
    {
        return new self(
            (string) ($data['name'] ?? ''),
            (string) ($data['host'] ?? ''),
            (string) ($data['type'] ?? self::TYPE_SSH),
            (string) ($data['account'] ?? ''),
            (int) ($data['port'] ?? 0),
        );
    }

    /** @return array<string, mixed> */
    public function toArray(): array
    {
        return [
            'name' => $this->name,
            'host' => $this->host,
            'type' => $this->type,
            'account' => $this->account,
            'port' => $this->port,
        ];
    }

    public function name(): string
    {
        return $this->name;
    }

    public function host(): string
    {
        return $this->host;
    }

    public function type(): string
    {
        return $this->type;
    }

    public function account(): string
    {
        return $this->account;
    }

    public function port(): int
    {
        return $this->port;
    }

    public function command(): array
    {
        if ($this->type === self::TYPE_WINRM) {
            $command = ['pwsh', '-NoLogo', '-Command', 'Enter-PSSession', '-ComputerName', $this->host];
            if ($this->account !== '') {
                $command[] = '-Credential';
                $command[] = $this->account;
            }
            return $command;
        }
        $command = ['ssh'];
        if ($this->port > 0) {
            $command[] = '-p';
            $command[] = (string) $this->port;
        }
        if ($this->account !== '') {
            $command[] = $this->account . '@' . $this->host;
        } else {
            $command[] = $this->host;
        }
        return $command;
    }

    public function validate(): bool
    {
        return $this->name !== ''
            && $this->host !== ''
            && in_array($this->type, [self::TYPE_SSH, self::TYPE_WINRM], true)
            && $this->port >= 0
            && $this->port <= 65535;
    }

    public function connect(): Session
    {
        $parts = $this->command();
        $program = array_shift($parts);
        return new Session($program, '', $parts);
    }
}
