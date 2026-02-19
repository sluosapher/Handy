# Handy Local Post-Processing Modification

This repository includes a modification to the original Handy project that adds local LLM post-processing for voice typing output.

## What Is Modified

The app uses **Microsoft Foundry Local** with a local **Phi-4-mini** model for post-processing.

When Handy starts, it will:

1. Check whether Foundry Local is installed
2. Install Foundry Local automatically if missing
3. Check whether the Phi-4-mini model is downloaded and loaded
4. Download and load Phi-4-mini automatically if needed

This design removes most manual setup steps for local post-processing.

## Why This Matters

All post-processing runs locally on the user's device. This improves privacy and data security because voice typing content is not sent to cloud services for post-processing.

## How To Use

Use the shortcut `Control + Shift + Space` to trigger voice typing with local post-processing.

## Download This Modified Handy App

To get this modified version, visit:

https://github.com/sluosapher/Handy
