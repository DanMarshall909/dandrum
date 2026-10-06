# Demo launcher

## Purpose

Provide a maintained command to discover, prepare and launch Dandrum developer demos.

## Requirements

### Requirement: Discoverable named demos

The repository SHALL provide an executable demo launcher that lists maintained names, descriptions, availability and preview versus embedded status, and rejects unknown names without starting a process.

#### Scenario: List and usage errors
- **WHEN** the launcher is invoked without a name, with a list/help option, or with an unknown name
- **THEN** known demos and their status are presented; list/help succeeds and an unknown name reports a usage error without launching a demo

### Requirement: Resolve demo checkouts

The launcher SHALL use its own checkout before compatible registered Dandrum worktrees, preserve paths containing spaces, work from another current directory, and report unavailable demo sources.

#### Scenario: Resolve and report source availability
- **WHEN** a requested demo exists locally, only in a registered compatible worktree, or nowhere
- **THEN** the local source wins, otherwise a deterministically selected compatible worktree is used, and missing sources produce an actionable failure without preparation

### Requirement: Prepare and launch demos

The launcher SHALL prepare only the selected native target or browser package, run it in the correct source directory and preserve user arguments.

#### Scenario: Launch native standalone apps
- **WHEN** a native demo is requested with arguments
- **THEN** its selected checkout is configured, its target is built and its standalone artifact receives those arguments, including arguments containing spaces

#### Scenario: Launch embedded React apps and browser previews
- **WHEN** an embedded React demo or browser preview is requested
- **THEN** the required locked package dependencies are prepared if absent, the embedded app enables its WebView backend, and the browser preview runs its dev command with forwarded arguments

### Requirement: Report operational failures

The launcher SHALL stop after preparation failure, report missing commands or artifacts, preserve child failure codes and handle interruption without a traceback.

#### Scenario: Failure and interruption
- **WHEN** configuration, dependency preparation, build or execution fails, a command or artifact is missing, or launch is interrupted
- **THEN** later steps do not run and the launcher returns a meaningful nonzero exit with a useful diagnostic; interruption returns 130

### Requirement: Maintain the demo inventory

The repository SHALL test that independently launchable JUCE demo targets and local React dev packages are represented by the catalog, and instruct maintainers to update the launcher when demos change.

#### Scenario: Detect missing registrations
- **WHEN** an independently launchable JUCE target or local React dev package is added without a catalog entry
- **THEN** the normal CTest maintenance check fails and names the missing demo source
