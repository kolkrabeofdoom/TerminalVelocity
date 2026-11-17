<?php

declare(strict_types=1);

namespace TerminalVelocity;

/**
 * Decodes Sixel graphics into RGBA pixels. The Rust implementation needed
 * 4096 line segments and a 16 MiB budget; PHP 2.0 does it in one array.
 */
final class SixelDecoder
{
    private const MAX_INPUT = 4 * 1024 * 1024;
    private const MAX_SIZE = 4096;

    private int $width = 0;
    private int $height = 0;
    private int $x = 0;
    private int $y = 0;

    /** @var array<int, int> */
    private array $palette = [];

    /** @var array<int, array<int, array{int,int,int,int}>> */
    private array $pixels = [];

    public function decode(string $data): ?array
    {
        if (strlen($data) > self::MAX_INPUT) {
            return null;
        }
        $this->reset();
        $length = strlen($data);
        for ($i = 0; $i < $length; $i++) {
            $char = $data[$i];
            if ($char >= '?' && $char <= '~') {
                $this->setPixel(ord($char) - 63);
            } elseif ($char === '!') {
                $i = $this->readRepeat($data, $i + 1);
            } elseif ($char === '#') {
                $i = $this->readColor($data, $i + 1);
            } elseif ($char === '$') {
                $this->x = 0;
            } elseif ($char === '-') {
                $this->x = 0;
                $this->y += 6;
            } elseif ($char === '"') {
                $this->setAspect($data, $i + 1);
            }
        }
        return $this->toImage();
    }

    private function reset(): void
    {
        $this->palette = [];
        $this->pixels = [];
        $this->x = 0;
        $this->y = 0;
        $this->width = 0;
        $this->height = 0;
        for ($index = 0; $index < 256; $index++) {
            $this->palette[$index] = ($index << 16) | ($index << 8) | $index;
        }
    }

    private function setPixel(int $color): void
    {
        if ($this->x >= self::MAX_SIZE || $this->y >= self::MAX_SIZE) {
            return;
        }
        $color &= 0xff;
        $packed = $this->palette[$color] ?? 0;
        $this->pixels[$this->y][$this->x] = [
            ($packed >> 16) & 0xff,
            ($packed >> 8) & 0xff,
            $packed & 0xff,
            255,
        ];
        $this->width = max($this->width, $this->x + 1);
        $this->height = max($this->height, $this->y + 1);
        $this->x++;
    }

    private function readRepeat(string $data, int $offset): int
    {
        $digits = '';
        $length = strlen($data);
        while ($offset < $length && ctype_digit($data[$offset])) {
            $digits .= $data[$offset];
            $offset++;
        }
        if ($digits === '' || $offset >= $length) {
            return $offset;
        }
        $count = (int) $digits;
        $char = $data[$offset];
        if ($char >= '?' && $char <= '~') {
            for ($i = 0; $i < min($count, 1024); $i++) {
                $this->setPixel(ord($char) - 63);
            }
        }
        return $offset;
    }

    private function readColor(string $data, int $offset): int
    {
        $length = strlen($data);
        $parts = [];
        while ($offset < $length && count($parts) < 5) {
            $char = $data[$offset];
            if ($char === ';' || $char === ',') {
                $parts[] = '';
            } elseif (ctype_digit($char)) {
                $parts[count($parts) - 1] = ($parts[count($parts) - 1] ?? '') . $char;
            } else {
                break;
            }
            $offset++;
        }
        if (count($parts) < 4) {
            return $offset;
        }
        $index = (int) ($parts[0] ?? 0);
        $space = (int) ($parts[1] ?? 1);
        $r = (int) ($parts[2] ?? 0);
        $g = (int) ($parts[3] ?? 0);
        $b = (int) ($parts[4] ?? 0);
        if ($space === 1) {
            $r = (int) round($r * 255 / 100);
            $g = (int) round($g * 255 / 100);
            $b = (int) round($b * 255 / 100);
        }
        $this->palette[$index & 0xff] = ($r << 16) | ($g << 8) | $b;
        return $offset;
    }

    private function setAspect(string $data, int $offset): int
    {
        $length = strlen($data);
        $digits = '';
        while ($offset < $length && ctype_digit($data[$offset])) {
            $digits .= $data[$offset];
            $offset++;
        }
        return $offset;
    }

    /** @return array{width:int,height:int,pixels:list<int>}|null */
    private function toImage(): ?array
    {
        if ($this->width === 0 || $this->height === 0) {
            return null;
        }
        $pixels = [];
        for ($y = 0; $y < $this->height; $y++) {
            for ($x = 0; $x < $this->width; $x++) {
                $pixel = $this->pixels[$y][$x] ?? [0, 0, 0, 0];
                foreach ($pixel as $channel) {
                    $pixels[] = $channel;
                }
            }
        }
        return [
            'width' => $this->width,
            'height' => $this->height,
            'pixels' => $pixels,
        ];
    }
}
