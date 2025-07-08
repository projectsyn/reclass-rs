import subprocess
import pytest

import reclass_rs


def test_buildinfo():
    buildinfo = reclass_rs.buildinfo()

    try:
        rustc_version = (
            subprocess.run(
                ["rustc", "--version"], stderr=subprocess.STDOUT, stdout=subprocess.PIPE
            )
            .stdout.decode("utf-8")
            .strip()
        )
    except FileNotFoundError:
        pytest.skip("No rustc compiler available in test environment")
    assert buildinfo == {"rustc_version": rustc_version}
