#! /usr/bin/env python
"""
Publish a new release for tego.

Usage: release.py <major|minor|patch>

This script will perform the following steps:
    - Bump the version number in Cargo.toml
    - Update the CHANGELOG.md with the new version
    - Commit and tag the changes
    - Publish the new version on crates.io

Depending on the version parameter, either the first, second or third component
of the version number is bumped.
"""

from datetime import date
import re
import subprocess
import sys
import textwrap

commit_message_template = textwrap.dedent("""\
    chore: release version {version}

    {changes}
""")

# Regex to grab the version number in Cargo.toml
version_regex = r'(?<=version\s=\s)"(?P<major>[0-9]+)\.(?P<minor>[0-9]+)\.(?P<patch>[0-9]+)"'


def bump_version(version, type):
    if type == "major":
        return (version[0] + 1, 0, 0)
    elif type == "minor":
        return (version[0], version[1] + 1, 0)
    elif type == "patch":
        return (version[0], version[1], version[2] + 1)
    else:
        raise ValueError(f"Unkown bump type '{type}'")


def usage():
    print(__doc__)
    sys.exit(1)


def main(argv):
    if len(argv) != 2:
        usage()

    if argv[1] not in ["major", "minor", "patch"]:
        usage()

    bump = argv[1]

    # Get version and update Cargo.toml
    with open("Cargo.toml", 'r+') as file:
        text = file.read()
        version_match = re.search(version_regex, text)
        major = int(version_match.group("major"))
        minor = int(version_match.group("minor"))
        patch = int(version_match.group("patch"))

        (major, minor, patch) = bump_version((major, minor, patch), bump)
        version = f"{major}.{minor}.{patch}"
        text = re.sub(version_regex, f'"{version}"', text)

        print("Updating to version", version)
        print("Press enter to continue or ctrl+C to abort")
        input()

        file.seek(0)
        file.write(text)

    # Update the CHANGELOG
    with open("CHANGELOG.md", "r+") as file:
        buffer = []
        changes = []
        in_changelog = False
        for line in file.readlines():
            buffer.append(line)

            if line.startswith("## ["):
                in_changelog = False

            if in_changelog:
                changes.append(line)

            if line == "## Unreleased\n":
                buffer += [
                    "\n",
                    f"## [{version}] - {date.today()}\n"
                ]
                in_changelog = True

        file.seek(0)
        file.writelines(buffer)

    # Commit and tag the changes
    commit_message = commit_message_template.format(
        version=version,
        changes="".join(changes)
    )

    subprocess.run([
        "git", "commit", "-m", commit_message, "-S",
        # With -- we can give the list of files directly to git commit
        # It will ignore the already staged changes
        "--", "CHANGELOG.md", "Cargo.toml",
    ], check=True)

    subprocess.run([
        "git", "tag", "-s",
        "-m", f"v{version}",
        "-m", "".join(changes),
        f"v{major}.{minor}.{patch}",
    ], check=True)

    print("Press enter to publish release", version)
    print("Last chance to abort via ctrl-c")
    input()

    subprocess.run(["git", "push", "--follow-tags"], check=True)
    subprocess.run(["cargo", "publish"], check=True)

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
