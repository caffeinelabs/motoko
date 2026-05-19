---
title: "Developer containers"
description: "Developer containers use Docker and VS Code to provide a portable, isolated local development environment for Motoko canisters."
sidebar:
  order: 2
---

Developer containers are a local development option that uses [Docker](https://www.docker.com/get-started/) and [VS Code](https://code.visualstudio.com/) to run local containerized environments. Containers are isolated from the rest of your local environment, and files within a container cannot be used by applications outside of the container unless explicitly mounted and given access. Developer containers are a good option for developers on Windows systems, since the ICP development environment is not natively supported on Windows.

Dev containers have additional benefits, including:

- **Reproducibility**: Containers make it easy to build and run in a consistent way across multiple computers since they contain the entire development environment within a portable format. Team members can interact with the same container without needing to set up their own personal developer environment and replicate the build themselves.

- **Security**: By default, containers do not have access to the other files or applications on your system, making project development secure. It's also possible to give restricted (e.g. read-only or write-only) access to specific files and directories.

## Using developer containers

To use developer containers, you must download and install:

- [Docker](https://www.docker.com/get-started/).

- [VS Code](https://code.visualstudio.com/).

- The [Dev Containers VS Code extension](https://containers.dev/supporting#dev-containers).

Make sure Docker is running, then navigate into your project's directory. Create a file called `devcontainer.json` that contains the following:

```json
{
  "name": "ICP Dev Environment",
  "image": "ghcr.io/dfinity/icp-dev-env-slim:latest",
  "forwardPorts": [4943, 5173],
  "portsAttributes": {
    "4943": {
      "label": "icp",
      "onAutoForward": "ignore"
    },
    "5173": {
      "label": "vite",
      "onAutoForward": "openBrowser"
    }
  },
  "customizations": {
    "vscode": {
      "extensions": ["dfinity-foundation.vscode-motoko"]
    }
  }
}
```

Then, start the developer container by opening the VS Code command palette (F1 or Ctrl+Shift+P), then select `Dev-Containers: Reopen in Container`.

