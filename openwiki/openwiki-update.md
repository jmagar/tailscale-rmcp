---
type: Operations Guide
title: "OpenWiki update workflow"
description: "Explains how the repository’s OpenWiki update workflow is configured and what changed in the latest commit set."
tags: [openwiki, operations, github-actions, automation]
timestamp: "2026-07-23T10:22:54Z"
---

The OpenWiki refresh pipeline is defined in [/.github/workflows/openwiki-update.yml](/.github/workflows/openwiki-update.yml).

## Current workflow behavior

- Trigger: `workflow_dispatch` and daily schedule `0 8 * * *`.
- Runtime: `ubuntu-latest`.
- Tooling: installs `openwiki` globally and runs:
  - `openwiki code --update --print`
- Provider configuration in the workflow now uses:
  - `OPENWIKI_PROVIDER=openrouter`
  - `OPENROUTER_API_KEY`
  - `OPENWIKI_MODEL_ID=z-ai/glm-5.2`
  - optional tracing env vars for LangSmith (`LANGSMITH_API_KEY`, `LANGCHAIN_PROJECT`, `LANGCHAIN_TRACING_V2`)

## What changed recently

Compared to the previous version, this run simplified and standardized the workflow:

- Removed the self-hosted runner + Tailscale/VPN connectivity preflight.
- Removed custom API-base preflight checks.
- Removed the explicit sourcing of host-specific `.env` files.
- Changed model/provider selection from `openai-compatible` to `openrouter`.
- Changed PR creation step to pin create-pull-request action by commit hash and include additional tracked files:
  - `AGENTS.md`
  - `CLAUDE.md`
  - `.github/workflows/openwiki-update.yml`
  - `openwiki`

## File-level impact

This page exists because recent source diffs directly modify docs automation behavior and instructions. It is directly linked from:

- [`/openwiki/quickstart.md`](quickstart.md)
- [`.github/workflows/openwiki-update.yml`](/.github/workflows/openwiki-update.yml)
- [`/CLAUDE.md`](/CLAUDE.md)

## Relationship notes

- The update workflow `runs` [OpenWiki](/.github/workflows/openwiki-update.yml) and `produces` generated repository docs under `/openwiki`.
- `/CLAUDE.md` now links OpenWiki operations back to `openwiki/quickstart.md`, so this page closes that loop for agent onboarding.
- This page complements existing project docs (for example `/docs/README.md` and `/README.md`) but focuses only on documentation lifecycle.