---
title: "tisket issue create should default to discovery status"
status: discovery
labels:
  - tisket
---

## Problem

`tisket issue create` defaults to `todo` status. New tiskets should start at `discovery` — work isn't ready to be picked up until discovery is complete and it's explicitly moved to `todo`.

## What needs to happen

Change the default status in `tisket issue create` from `todo` to `discovery`.
