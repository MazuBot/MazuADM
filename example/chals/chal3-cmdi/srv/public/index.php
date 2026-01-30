<?php
declare(strict_types=1);

// Router for PHP's built-in server:
// Allow static files through when they exist.
if (PHP_SAPI === 'cli-server') {
  $path = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH) ?: '/';
  $file = __DIR__ . $path;
  if ($path !== '/' && is_file($file)) {
    return false;
  }
}

function json_response(array $data, int $status = 200): void {
  http_response_code($status);
  header('Content-Type: application/json; charset=utf-8');
  echo json_encode($data, JSON_UNESCAPED_SLASHES) . "\n";
}

function text_response(string $body, int $status = 200, string $content_type = 'text/plain; charset=utf-8'): void {
  http_response_code($status);
  header('Content-Type: ' . $content_type);
  echo $body;
}

function ensure_flag_generator(): void {
  $pid_file = '/tmp/flaggen.pid';
  $lock_file = '/tmp/flaggen.lock';

  $lock = @fopen($lock_file, 'c');
  if ($lock === false) {
    return;
  }

  if (!@flock($lock, LOCK_EX)) {
    @fclose($lock);
    return;
  }

  $pid = '';
  if (is_file($pid_file)) {
    $pid = trim((string)@file_get_contents($pid_file));
  }

  if ($pid !== '' && ctype_digit($pid) && is_dir('/proc/' . $pid)) {
    @flock($lock, LOCK_UN);
    @fclose($lock);
    return;
  }

  $cmd = <<<'CMD'
bash -c 'while true; do ts=$(date +%s); bucket=$((ts/5*5)); tmp="/tmp/flag.$$"; printf "FLAG{TS_%s}\n" "$bucket" > "$tmp"; chmod 444 "$tmp"; mv -f "$tmp" /tmp/flag; sleep 5; done' > /dev/null 2>&1 & echo $!
CMD;

  $out = trim((string)@shell_exec($cmd));
  if ($out !== '' && ctype_digit($out)) {
    @file_put_contents($pid_file, $out . "\n");
  }

  @flock($lock, LOCK_UN);
  @fclose($lock);
}

function route(): void {
  // Start/ensure a background flag generator. Flag rotates every 5 seconds.
  // The exploit reads it via `cat /flag` (symlinked to /tmp/flag).
  ensure_flag_generator();

  $method = $_SERVER['REQUEST_METHOD'] ?? 'GET';
  $path = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH) ?: '/';

  if ($method === 'GET' && $path === '/') {
    $html = file_get_contents(__DIR__ . '/page.html');
    if ($html === false) {
      text_response("missing page.html\n", 500);
      return;
    }
    text_response($html, 200, 'text/html; charset=utf-8');
    return;
  }

  if ($method === 'GET' && $path === '/health') {
    json_response(['status' => 'ok']);
    return;
  }

  if ($method === 'GET' && $path === '/source') {
    header('Content-Type: text/plain; charset=utf-8');
    readfile(__FILE__);
    echo "\n\n--- page.html ---\n";
    readfile(__DIR__ . '/page.html');
    return;
  }

  if ($method === 'POST' && $path === '/api/ping') {
    $raw = file_get_contents('php://input') ?: '';
    $data = json_decode($raw, true);
    $host = '';

    if (is_array($data) && isset($data['host']) && is_string($data['host'])) {
      $host = $data['host'];
    } elseif (isset($_POST['host']) && is_string($_POST['host'])) {
      $host = $_POST['host'];
    }

    $host = trim($host);
    if ($host === '') {
      json_response(['ok' => false, 'error' => 'missing host'], 400);
      return;
    }

    // Intentionally vulnerable: untrusted input is concatenated into a shell command.
    // Output is suppressed, making this a "blind" injection.
    $cmd = "timeout 1 ping -c 1 -W 1 " . $host . " > /dev/null 2>&1";
    shell_exec($cmd);

    json_response(['ok' => true, 'queued' => true]);
    return;
  }

  json_response(['ok' => false, 'error' => 'not found'], 404);
}

route();
