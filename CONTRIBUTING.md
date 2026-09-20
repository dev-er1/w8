# Contributing to W8
First of all, thank you for your interest in W8! I'm glad that you want to contribute to the project.
Below are instructions on how you can help.

## How to Contribute
1. Read the [documentation](docs/Architecture/Architecture.md).
2. Find an issue you want to solve or a feature you want to add.
3. If the change affects W8's architecture, ISA, or bytecode format, create an issue to discuss it before starting work.
4. Fork the repository and create a new branch for your work.
5. Make your changes.
6. Make sure the code passes CI and all tests.
7. If the changes affect W8's behavior or architecture, update the documentation.
8. Submit a Pull Request describing your changes.

## Pull Request
A Pull Request should contain:
- a brief description of the changes;
- a description of the problem the change solves;
- tests for new functionality or bug fixes, if possible;
- documentation updates if the public behavior of W8 has changed.

Try not to mix multiple unrelated changes in a single Pull Request.

## Code Style
Before submitting a Pull Request, make sure that the code follows the project's standards. To run CI locally, execute one of the scripts below depending on your shell:
- [PowerShell CI script](scripts/ci/ci.ps1);
- [Sh CI script](scripts/ci/ci.sh).

## Writing Documentation
When writing documentation (whether in the code or in separate documents), it is advisable to avoid colloquial expressions.

Code comments must be written exclusively in **English**.

## Issues
- Bug report:

  When creating a bug report, include:
  - a description of the problem;
  - steps to reproduce it;
  - expected behavior;
  - actual behavior;
  - optionally, a screenshot showing the error;
  - the W8 version.

- Feature request:

  Describe:
  - the problem the proposed feature solves;
  - the expected behavior;
  - an example of its usage, if necessary.
