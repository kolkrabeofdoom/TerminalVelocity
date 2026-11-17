<?php

declare(strict_types=1);

require __DIR__ . '/TerminalBuffer.php';
require __DIR__ . '/AnsiParser.php';
require __DIR__ . '/SixelDecoder.php';
require __DIR__ . '/Pane.php';
require __DIR__ . '/Session.php';
require __DIR__ . '/Editor.php';
require __DIR__ . '/Config.php';
require __DIR__ . '/Remote.php';
require __DIR__ . '/Renderer.php';
require __DIR__ . '/Search.php';
require __DIR__ . '/CommandPalette.php';
require __DIR__ . '/App.php';
require __DIR__ . '/TerminalVelocity.php';

use TerminalVelocity\AnsiParser;
use TerminalVelocity\App;
use TerminalVelocity\Cell;
use TerminalVelocity\CommandPalette;
use TerminalVelocity\Config;
use TerminalVelocity\Editor;
use TerminalVelocity\Pane;
use TerminalVelocity\PaneTree;
use TerminalVelocity\Remote;
use TerminalVelocity\Renderer;
use TerminalVelocity\Search;
use TerminalVelocity\Session;
use TerminalVelocity\SixelDecoder;
use TerminalVelocity\TerminalBuffer;
use TerminalVelocity\TerminalVelocity;

/** @var list<string> $failures */
$failures = [];
/** @var int $checks */
$checks = 0;

function check(string $name, bool $condition): void
{
    global $failures, $checks;
    $checks++;
    if (!$condition) {
        $failures[] = $name;
    }
}

// TerminalBuffer
$buffer = new TerminalBuffer(10, 3, 5);
check('grid dimensions', $buffer->dimensions() === [10, 3]);
check('blank grid', $buffer->visibleLines() === ['', '', '']);
$buffer->put('A', 1, 0, Cell::FLAG_BOLD);
check('cursor advanced', $buffer->cursor() === [1, 0]);
check('cell written', $buffer->visibleLines()[0] === 'A');
$buffer->carriageReturn();
check('carriage return', $buffer->cursor() === [0, 0]);
$buffer->backspace();
check('backspace at origin', $buffer->cursor() === [0, 0]);
$buffer->setCursor(9, 2);
$buffer->put('Z', 0, 0, 0);
check('line feed at edge', $buffer->cursor() === [10, 2]);
$buffer->eraseInDisplay(2);
check('erase display', $buffer->visibleLines() === ['', '', '']);
$buffer->resize(4, 2);
check('resize', $buffer->dimensions() === [4, 2]);
$buffer->saveCursor();
$buffer->setCursor(0, 0);
$buffer->restoreCursor();
check('cursor save/restore', $buffer->cursor() === [0, 0]);

// ANSI parser
$buffer = new TerminalBuffer(20, 5);
$parser = new AnsiParser($buffer);
$parser->feed("hello\x1b[2;3Hworld");
check('cursor positioning', $buffer->visibleLines()[1] === '  world');
$parser->feed("\x1b[31mred\x1b[0m");
check('colour escape consumed', str_contains(implode('', $buffer->visibleLines()), 'red'));
$parser->feed("\x1b[2J\x1b[H");
check('clear and home', $buffer->cursor() === [0, 0]);
$parser->feed("\x1b7\x1b8");
check('escape save/restore', $buffer->cursor() === [0, 0]);

// Sixel decoder
$sixel = new SixelDecoder();
check('empty sixel', $sixel->decode('') === null);
$image = $sixel->decode('~');
check('single pixel', $image !== null && $image['width'] === 1 && $image['height'] === 1);
check('rgba channels', $image !== null && count($image['pixels']) === 4);
check('newline in sixel', $sixel->decode('~$-~') !== null);

// Panes
$tree = new PaneTree(80, 24);
check('single leaf', count($tree->leaves()) === 1);
$second = $tree->split('horizontal', $tree->leaves()[0]);
check('two leaves', count($tree->leaves()) === 2);
check('unique ids', $tree->leaves()[0]->id() !== $second->id());
$pane = new Pane(99, 10, 2);
$pane->write("alert\x07");
check('activity marker', $pane->hasActivity());
$pane->setFocus(true);
check('focus clears marker', !$pane->hasActivity());
$pane->resize(20, 5);
check('pane resize', $pane->buffer()->dimensions() === [20, 5]);

// Session (no process started)
$session = new Session('pwsh', '/tmp', ['-NoLogo']);
check('session program', $session->program() === 'pwsh');
check('session directory', $session->workingDirectory() === '/tmp');
check('session idle', !$session->isRunning());
check('session exit code', $session->exitCode() === -1);

// Editor
$editor = new Editor();
$editor->insert('abc');
check('editor insert', $editor->lines() === ['abc']);
$editor->newline();
$editor->insert('def');
check('editor newline', $editor->lines() === ['abc', 'def']);
$editor->deleteBefore();
check('editor backspace', $editor->lines() === ['abc', 'de']);
$editor->undo();
check('editor undo', $editor->lines() === ['abc', 'def']);
$editor->redo();
check('editor redo', $editor->lines() === ['abc', 'de']);
check('editor find', $editor->find('abc') === [0]);
check('editor dirty', $editor->isDirty());
check('editor encoding', $editor->encoding() === 'UTF-8');
check('editor line ending', $editor->lineEnding() === 'LF');

// Config
$config = new Config(sys_get_temp_dir());
check('default theme', $config->theme() === 'Aurora');
check('profile save', $config->saveProfile('demo', ['tabs' => []]));
check('profile lookup', $config->profile('demo') !== null);
check('profile names', in_array('demo', $config->profileNames(), true));
check('profile delete', $config->deleteProfile('demo'));
check('profile removed', $config->profile('demo') === null);
$config->setTheme('Phosphor');
check('theme set', $config->theme() === 'Phosphor');
check('invalid theme ignored', !$config->setTheme('Nope') || $config->theme() === 'Phosphor');
$config->setIntensity(2.0);
check('intensity clamped', $config->intensity() === 1.0);
check('connection add', $config->addConnection(['host' => 'localhost']));
check('connections stored', count($config->connections()) === 1);

// Remote
$ssh = new Remote('prod', 'example.test', Remote::TYPE_SSH, 'ops', 2222);
check('ssh command', $ssh->command() === ['ssh', '-p', '2222', 'ops@example.test']);
check('ssh valid', $ssh->validate());
$winrm = new Remote('win', 'srv', Remote::TYPE_WINRM);
check('winrm command', $winrm->command()[0] === 'pwsh');
check('invalid port', !(new Remote('x', 'y', Remote::TYPE_SSH, '', 70000))->validate());
$roundTrip = Remote::fromArray($ssh->toArray());
check('remote round trip', $roundTrip->host() === 'example.test');

// Renderer
$renderer = new Renderer();
check('default preset', $renderer->preset() === Renderer::PRESET_AURORA);
check('preset set', $renderer->setPreset(Renderer::PRESET_PHOSPHOR));
check('preset invalid', !$renderer->setPreset('Invalid'));
check('renderer output', str_contains($renderer->render($buffer), "\x1b["));

// Search
$search = new Search(['alpha', 'beta alpha', 'gamma']);
$search->setQuery('alpha');
check('search matches', $search->count() === 2);
check('search next', $search->next() === 0);
check('search wrap', $search->next() === 1 && $search->next() === 0);
check('search previous', $search->previous() !== null);
$search->close();
check('search closed', $search->count() === 0);

// Palette
$palette = new CommandPalette();
$palette->register('New tab', static function (): void {
});
check('palette count', $palette->count() === 1);
check('palette filter', $palette->filter('tab') === ['New tab']);
check('palette execute', $palette->execute('New tab'));
check('palette unknown', !$palette->execute('Missing'));

// App and facade
$app = new App($config, $renderer);
$app->start();
check('app running', $app->activePane() !== null);
check('app render', is_string($app->render()));
$app->beginSearch('x');
check('app search', $app->search() !== null);
$app->endSearch();
check('app search closed', $app->search() === null);
$app->stop();

$tv = new TerminalVelocity(sys_get_temp_dir());
check('version', TerminalVelocity::version() === '2.0.0');
check('runtime', TerminalVelocity::RUNTIME === 'PHP 2.0');
check('facade commands', count($tv->commands()) >= 5);
check('facade smoke test', $tv->smokeTest());

// Result
if ($failures !== []) {
    fwrite(STDERR, sprintf("selftest: %d/%d checks passed\n", $checks - count($failures), $checks));
    foreach ($failures as $failure) {
        fwrite(STDERR, "  FAILED: {$failure}\n");
    }
    exit(1);
}

echo "selftest: {$checks}/{$checks} checks passed\nOK\n";
