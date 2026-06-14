
import { execSync } from 'node:child_process';

const port = process.argv[2] ?? '1420';

function freeWindows() {
  try {
    const out = execSync(`netstat -ano | findstr :${port}`, {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    const pids = [
      ...new Set(
        out
          .split('\n')
          .map((line) => line.trim().split(/\s+/).at(-1))
          .filter((pid) => pid && pid !== '0' && /^\d+$/.test(pid))
      ),
    ];
    for (const pid of pids) {
      try {
        execSync(`taskkill /PID ${pid} /F`, { stdio: 'ignore' });
        console.log(`Freed port ${port}: stopped PID ${pid}`);
      } catch {

      }
    }
  } catch {

  }
}

function freeUnix() {
  try {
    const out = execSync(`lsof -ti :${port}`, { encoding: 'utf8' }).trim();
    if (!out) return;
    for (const pid of out.split('\n')) {
      try {
        process.kill(Number(pid), 'SIGTERM');
        console.log(`Freed port ${port}: stopped PID ${pid}`);
      } catch {

      }
    }
  } catch {

  }
}

if (process.platform === 'win32') {
  freeWindows();
} else {
  freeUnix();
}
