# @opencode-vibe/protocol

Shared Types and Parsing logic for the Opencode Vibe Kanban. This module is pure logic and contains no I/O operations.

## Inputs

*   **Raw Strings**: The `Parser` class takes raw strings as input. These strings may contain ANSI codes or other formatting.

## Outputs

*   **Message Object**: The `Parser` returns a structured `Message` object.
    *   `raw`: The original raw string.
    *   `content`: The parsed content (currently identical to raw for MVP).

## Logic

The module provides a `Parser` class that implements the parsing logic.

### Parser

*   **Method**: `parse(raw: string): Message`
*   **Behavior**: Converts a raw input string into a `Message` object.

## Usage

```typescript
import { Parser } from '@opencode-vibe/protocol';

const parser = new Parser();
const message = parser.parse("\x1b[32mThinking...\x1b[0m");

console.log(message.raw);
console.log(message.content);
```
