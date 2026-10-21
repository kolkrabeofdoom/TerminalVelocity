# Local SIXEL demonstration; no downloads or additional tools required.
$esc = [char]27
$image = [Text.StringBuilder]::new()
[void]$image.Append("${esc}P0;1q`"1;1;240;120")
$colors = @('15;65;90', '40;85;65', '95;75;25', '90;35;45')
for ($index = 0; $index -lt $colors.Count; $index++) {
    [void]$image.Append("#$index;2;$($colors[$index])")
}
for ($band = 0; $band -lt 20; $band++) {
    for ($stripe = 0; $stripe -lt 4; $stripe++) {
        $color = ($stripe + [int][Math]::Floor($band / 5)) % 4
        [void]$image.Append("#$color!60~")
    }
    if ($band -lt 19) { [void]$image.Append('-') }
}
[void]$image.Append("${esc}\")
[Console]::WriteLine('TerminalVelocity: 240 x 120 Pixel, vier Farben, kein galaktischer Umweg.')
[Console]::Write($image.ToString())
[Console]::WriteLine('Sixel-Demo beendet. Diese Zeile steht unter dem Bild.')
