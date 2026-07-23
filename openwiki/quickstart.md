---
type: Quickstart
title: "OpenWiki quickstart for tailscale-rmcp"
description: "Start here to understand how this repository keeps generated documentation in sync, what changed in the OpenWiki automation, and where to go for update procedures."
tags: [openwiki, tailscale-rmcp, documentation]
timestamp: "2026-07-23T10:22:54Z"
---

This repository’s OpenWiki entrypoint is `/openwiki/quickstart.md`; use it before editing generated pages.

Recent updates touched the OpenWiki automation and agent guidance, so the docs now explicitly document how updates are triggered and committed.

## What this wiki tracks

The generated knowledge pages track:

- The automated update pipeline in `/.github/workflows/openwiki-update.yml`.
- How agent instructions in `/CLAUDE.md` refer to OpenWiki refresh habits.
- Stable links to upstream source docs such as `/README.md`, `/docs/README.md`, `/docs/SETUP.md`, and `/docs/INVENTORY.md`.

## Start here

- **Update how docs are generated:** read [`/openwiki/openwiki-update.md`](/openwiki/openwiki-update.md).
- **Understand source truth for behavior:** start at [`/README.md`](/README.md), then [`/docs/README.md`](/docs/README.md).
- **Check current operational constraints before touching docs:** [`/CLAUDE.md`](/CLAUDE.md) includes the latest guidance on OpenWiki workflow expectations.

## OpenWiki maintenance flow

`/.github/workflows/openwiki-update.yml` runs a scheduled/manual workflow and executes `openwiki code --update --print` to regenerate documentation content in this directory, then opens a pull request with generated docs and policy files. See the [OpenWiki update playbook](openwiki-update.md) for exact env vars, scheduling, and PR behavior.

If you are doing a source-level behavior change unrelated to docs tooling, update source docs in `/docs` first, then run the update flow.

## Backlog

- `openwiki/source-map` (anchor: `/docs/INVENTORY.md`) — deferred because no dedicated static source-map section exists yet and this repo-level surface is currently maintained in `/docs/README.md` instead.
