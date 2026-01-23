"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Parser = void 0;
class Parser {
    parse(raw) {
        return {
            raw,
            content: raw, // For MVP, content is just the raw string
        };
    }
}
exports.Parser = Parser;
