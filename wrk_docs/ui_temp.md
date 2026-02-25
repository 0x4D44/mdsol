# UI Interactions and Input Handling System

## Overview

This document provides a comprehensive analysis of the user interface interaction and input handling system in the Solitaire application, focusing on mouse events, keyboard input, drag-and-drop operations, and hit testing mechanisms.

**File:** `C:\language\mdsol\src\main.rs`  
**Date:** 2025-11-06

---

## Table of Contents

1. [Core Data Structures](#core-data-structures)
2. [Mouse Event Handling](#mouse-event-handling)
3. [Drag-and-Drop System](#drag-and-drop-system)
4. [Hit Testing](#hit-testing)
5. [Keyboard Input and Accelerators](#keyboard-input-and-accelerators)
6. [Double-Click Handling](#double-click-handling)
7. [Focus Management](#focus-management)
8. [State Management](#state-management)

---

## Core Data Structures

### HitTarget (Lines 1909-1918)

The `HitTarget` enum represents clickable regions in the game interface:

