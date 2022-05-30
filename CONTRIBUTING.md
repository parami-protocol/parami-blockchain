# Contributing to Parami

## We Use Github Flow

Pull requests are the best way to propose changes to the codebase.

1. Fork the repo and create your branch from the main branch.
1. If you've added code that should be tested, add tests.
1. Ensure the test suite passes.
1. Make sure your code lints.
1. Issue your pull request.

## Pull request process

1. All required checks have completed successfully.
    1. CI build (Actions / ru-build)
    1. CI tests (Actions / ru-test)
    1. Coverage (by using [codecov.io](https://app.codecov.io/gh/parami-protocol/parami-blockchain/))
    1. Linting (Actions / ru-lint)
1. At least one reviewer approve your latest changes.
1. A maintainer comments `bors r+` adding it to the merge queue.

## License

By contributing, you agree that your contributions will be licensed under its [GNU GENERAL PUBLIC LICENSE Version 3](LICENSE).
