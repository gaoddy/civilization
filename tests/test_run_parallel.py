import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "run_parallel.py"


def run_parallel(*commands: str, jobs: int | None = None) -> subprocess.CompletedProcess:
    args = [sys.executable, str(SCRIPT)]
    if jobs is not None:
        args.extend(["--jobs", str(jobs)])
    args.extend(commands)
    return subprocess.run(args, capture_output=True, text=True)


class RunParallelTest(unittest.TestCase):
    def test_runs_every_command_and_reports_success(self):
        result = run_parallel("echo alpha", "echo beta", jobs=2)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("alpha", result.stdout)
        self.assertIn("beta", result.stdout)
        self.assertNotIn("失败", result.stderr)

    def test_single_job_runs_serially(self):
        result = run_parallel("echo one", "echo two", jobs=1)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertLess(result.stdout.index("one"), result.stdout.index("two"))

    def test_failure_keeps_running_and_exits_nonzero(self):
        result = run_parallel("echo before", "exit 3", "echo after", jobs=3)

        self.assertEqual(result.returncode, 1)
        self.assertIn("after", result.stdout)
        self.assertIn("退出 3", result.stdout)
        self.assertIn("exit 3", result.stderr)

    def test_truncates_long_output_from_the_front(self):
        result = run_parallel(
            f"{sys.executable} -c \"print('x' * 20000); print('LAST')\"", jobs=1
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("已截断前", result.stdout)
        self.assertIn("LAST", result.stdout)
        self.assertLess(len(result.stdout), 20000)

    def test_rejects_invalid_jobs(self):
        result = run_parallel("echo no", jobs=0)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--jobs", result.stderr)


if __name__ == "__main__":
    unittest.main()
