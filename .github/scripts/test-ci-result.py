"""Exercise the merge gate against failed, cancelled, skipped and absent results."""

import itertools
from pathlib import Path
import subprocess
import unittest


SCRIPT = Path(__file__).with_name("ci-result.sh")


class CiResultTests(unittest.TestCase):
    def accepts(self, *arguments):
        result = subprocess.run(
            ["bash", str(SCRIPT), *arguments], capture_output=True, check=False
        )
        return result.returncode == 0

    def test_full_requires_every_platform_to_succeed(self):
        states = ("success", "failure", "cancelled", "skipped", "")
        for results in itertools.product(states, repeat=3):
            with self.subTest(results=results):
                self.assertEqual(
                    self.accepts("true", *results),
                    results == ("success", "success", "success"),
                )

    def test_quick_requires_linux_and_omits_other_platforms(self):
        states = ("success", "failure", "cancelled", "skipped", "")
        for results in itertools.product(states, repeat=3):
            with self.subTest(results=results):
                self.assertEqual(
                    self.accepts("false", *results),
                    results == ("success", "skipped", "skipped"),
                )

    def test_invalid_mode_or_incomplete_arguments_cannot_pass(self):
        for arguments in [(), ("true",), ("false", "success"),
                          ("", "success", "success", "success"),
                          ("unexpected", "success", "success", "success"),
                          ("true", "success", "success", "success", "extra")]:
            with self.subTest(arguments=arguments):
                self.assertFalse(self.accepts(*arguments))


if __name__ == "__main__":
    unittest.main()
