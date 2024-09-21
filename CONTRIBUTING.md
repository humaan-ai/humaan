# Contribute to Humaan - Let's Build AI for Humanity

First of all, **thank you** for considering contributing to Humaan! Your involvement is vital to the success of this open-source project. By collaborating, we can build AI tools that truly benefit humanity.

## Table of Contents

- [Contribute to Humaan - Let's Build AI for Humanity](#contribute-to-humaan---lets-build-ai-for-humanity)
  - [Table of Contents](#table-of-contents)
  - [How Can You Contribute?](#how-can-you-contribute)
  - [Getting Started](#getting-started)
    - [Prerequisites](#prerequisites)
    - [Setting Up the Development Environment](#setting-up-the-development-environment)
  - [Contribution Guidelines](#contribution-guidelines)
    - [Reporting Bugs](#reporting-bugs)
    - [Suggesting Enhancements](#suggesting-enhancements)
    - [Submitting Changes](#submitting-changes)
  - [Development Workflow](#development-workflow)
    - [Branching Strategy](#branching-strategy)
    - [Creating a New Branch](#creating-a-new-branch)
    - [Coding Standards](#coding-standards)
    - [Commit Messages](#commit-messages)
    - [Pull Request Process](#pull-request-process)
  - [Community and Support](#community-and-support)
  - [License](#license)


---

## How Can You Contribute?

There are many ways to contribute to Humaan:

- **Report Bugs**: Identify and report issues to help improve the project.
- **Suggest Enhancements**: Propose new features or improvements.
- **Code Contributions**: Fix bugs, implement features, or improve documentation.
- **Documentation**: Enhance existing documentation or add new sections.
- **Community Engagement**: Participate in discussions, answer questions, and help other community members.

## Getting Started

### Prerequisites

Before you begin, ensure you have met the following requirements:

- **Operating System**: Windows, macOS, or Linux
- **Node.js**: Version 16 or higher
- **Rust and Cargo**: For building native modules
- **Tauri CLI**: For building the desktop application
- **Git**: For version control

### Setting Up the Development Environment

`WIP`

## Contribution Guidelines

### Reporting Bugs

If you find a bug, please open an issue with the following details:

- **Summary**: A concise description of the bug.
- **Steps to Reproduce**: Detailed steps to replicate the issue.
- **Expected Behavior**: What you expected to happen.
- **Actual Behavior**: What actually happened.
- **Screenshots**: If applicable, include screenshots to illustrate the problem
- **Environment**: OS, Node.js version, any other relevant details.

[Open a Bug Report](link_to_open_issue)

### Suggesting Enhancements

To suggest new features or improvements:

- Use Cases: Explain the problem and how your suggestion solves it.
- Proposal: Describe your suggested enhancement in detail.
- Alternatives: Mention any alternative solutions you've considered.

[Suggest an Enhancement](link_to_discussion)

### Submitting Changes

Before submitting changes:

- Ensure your code follows the project's coding standards.
- Test your changes thoroughly.
- Update documentation if necessary.
  
## Development Workflow

### Branching Strategy
- **Main Branch (`main`)**: Contains stable code ready for release.
- **Development Branch (`develop`)**: Used for ongoing development.
- **Feature Branches (`feature/your-feature-name`)**: For new features or enhancements.
- **Bugfix Branches (`fix/your-bugfix-name`)**: For bug fixes.
  
### Creating a New Branch
Create a feature or bugfix branch from develop:

```bash
git checkout develop
git pull
git checkout -b feature/your-feature-name
```

### Coding Standards

- **Language**: Follow JavaScript/TypeScript best practices.
- **Formatting**: Use Prettier for code formatting.
- **Linting**: Ensure code passes ESLint checks.

Run linting and formatting checks:

```bash
npm run lint
npm run format
```

### Commit Messages

- Format: Use the Conventional Commits style.
- Structure: `<type>(<scope>): <subject>`
 
  Examples:
  - `feat(ui): add new theme selector`
  - `fix(api): correct data fetching logic`

### Pull Request Process

1. Push Your Branch
2. Open a Pull Request
   - Go to your fork on GitHub.
   - Click on "Compare & pull request".
   - Fill out the pull request template.
3. Review and Feedback
   - Be responsive to comments and requested changes.
   - Discuss any feedback or questions in the pull request.
4. Merge
   - A maintainer will merge your pull request once it meets all requirements.

## Community and Support

- Discord: Join our [community server](https://discord.gg/x7GEdb3a) for real-time discussions.
- GitHub Discussions: Participate in [community discussions](https://github.com/orgs/humaan-ai/discussions) on GitHub.

## License
By contributing, you agree that your contributions will be licensed under the MIT License.

> Thank you for helping make Humaan better! Your contributions are what make open-source projects thrive. Together, we're building AI for humanity.
