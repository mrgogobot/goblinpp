package org.goblinpp.jetbrains;

import com.intellij.lexer.LexerBase;
import com.intellij.psi.TokenType;
import com.intellij.psi.tree.IElementType;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

public final class GoblinLexer extends LexerBase {
    private static final int NORMAL = 0;
    private static final int RUST = 1;

    private CharSequence buffer = "";
    private int endOffset;
    private int tokenStart;
    private int tokenEnd;
    private IElementType tokenType;
    private int state;

    @Override
    public void start(@NotNull CharSequence buffer, int startOffset, int endOffset, int initialState) {
        this.buffer = buffer;
        this.endOffset = endOffset;
        this.tokenStart = startOffset;
        this.tokenEnd = startOffset;
        this.state = initialState;
        locateToken();
    }

    @Override
    public int getState() {
        return state;
    }

    @Override
    public @Nullable IElementType getTokenType() {
        return tokenType;
    }

    @Override
    public int getTokenStart() {
        return tokenStart;
    }

    @Override
    public int getTokenEnd() {
        return tokenEnd;
    }

    @Override
    public void advance() {
        tokenStart = tokenEnd;
        locateToken();
    }

    @Override
    public @NotNull CharSequence getBufferSequence() {
        return buffer;
    }

    @Override
    public int getBufferEnd() {
        return endOffset;
    }

    private void locateToken() {
        if (tokenStart >= endOffset) {
            tokenType = null;
            tokenEnd = endOffset;
            return;
        }

        if (state == RUST) {
            locateRustToken();
            return;
        }

        char current = buffer.charAt(tokenStart);
        if (Character.isWhitespace(current)) {
            tokenEnd = tokenStart + 1;
            while (tokenEnd < endOffset && Character.isWhitespace(buffer.charAt(tokenEnd))) {
                tokenEnd++;
            }
            tokenType = TokenType.WHITE_SPACE;
            return;
        }
        if (current == '#') {
            tokenEnd = untilNewline(tokenStart);
            tokenType = GoblinTokenTypes.COMMENT;
            return;
        }
        if (current == '"') {
            locateString();
            return;
        }
        if (isNumberStart(tokenStart)) {
            locateNumber();
            return;
        }
        if (isIdentifierStart(current)) {
            locateIdentifier();
            return;
        }
        if (isOperator(current)) {
            locateOperator();
            return;
        }
        IElementType punctuation = punctuationType(current);
        if (punctuation != null) {
            tokenEnd = tokenStart + 1;
            tokenType = punctuation;
            return;
        }
        tokenEnd = tokenStart + 1;
        tokenType = GoblinTokenTypes.BAD_CHARACTER;
    }

    private void locateRustToken() {
        int cursor = tokenStart;
        while (cursor < endOffset && (buffer.charAt(cursor) == ' ' || buffer.charAt(cursor) == '\t')) {
            cursor++;
        }
        if (startsWith(cursor, "RUST_INLINE_END") && atIdentifierBoundary(cursor + 15)) {
            if (cursor > tokenStart) {
                tokenEnd = cursor;
                tokenType = TokenType.WHITE_SPACE;
            } else {
                tokenEnd = cursor + 15;
                tokenType = GoblinTokenTypes.DIRECTIVE;
                state = NORMAL;
            }
            return;
        }
        tokenEnd = untilNewline(tokenStart);
        if (tokenEnd == tokenStart && tokenEnd < endOffset) {
            tokenEnd++;
        }
        tokenType = GoblinTokenTypes.INLINE_RUST;
    }

    private void locateString() {
        int cursor = tokenStart + 1;
        boolean escaped = false;
        while (cursor < endOffset) {
            char value = buffer.charAt(cursor);
            if (value == '\n' || value == '\r') {
                break;
            }
            cursor++;
            if (escaped) {
                escaped = false;
            } else if (value == '\\') {
                escaped = true;
            } else if (value == '"') {
                break;
            }
        }
        tokenEnd = cursor;
        tokenType = GoblinTokenTypes.STRING;
    }

    private void locateNumber() {
        int cursor = tokenStart;
        if (buffer.charAt(cursor) == '.') {
            cursor++;
        }
        while (cursor < endOffset && Character.isDigit(buffer.charAt(cursor))) {
            cursor++;
        }
        if (cursor < endOffset && buffer.charAt(cursor) == '.') {
            cursor++;
            while (cursor < endOffset && Character.isDigit(buffer.charAt(cursor))) {
                cursor++;
            }
        }
        if (cursor < endOffset && (buffer.charAt(cursor) == 'e' || buffer.charAt(cursor) == 'E')) {
            int exponent = cursor + 1;
            if (exponent < endOffset && (buffer.charAt(exponent) == '+' || buffer.charAt(exponent) == '-')) {
                exponent++;
            }
            int digits = exponent;
            while (exponent < endOffset && Character.isDigit(buffer.charAt(exponent))) {
                exponent++;
            }
            if (exponent > digits) {
                cursor = exponent;
            }
        }
        tokenEnd = cursor;
        tokenType = GoblinTokenTypes.NUMBER;
    }

    private void locateIdentifier() {
        int cursor = tokenStart + 1;
        while (cursor < endOffset && isIdentifierContinue(buffer.charAt(cursor))) {
            cursor++;
        }
        tokenEnd = cursor;
        String spelling = buffer.subSequence(tokenStart, tokenEnd).toString();
        if ("RUST_INLINE_BEGIN".equals(spelling)) {
            tokenType = GoblinTokenTypes.DIRECTIVE;
            state = RUST;
            return;
        }
        if ("RUST_INLINE_END".equals(spelling) || "GO_PARANOID".equals(spelling) || "GO_LOOP_BUDGET".equals(spelling)) {
            tokenType = GoblinTokenTypes.DIRECTIVE;
            return;
        }
        if ("true".equals(spelling) || "false".equals(spelling)) {
            tokenType = GoblinTokenTypes.BOOLEAN;
            return;
        }
        String kind = GoblinVocabulary.kindOf(spelling);
        tokenType = switch (kind == null ? "" : kind) {
            case "function" -> GoblinTokenTypes.FUNCTION;
            case "constant" -> GoblinTokenTypes.CONSTANT;
            case "unit" -> GoblinTokenTypes.UNIT;
            case "statement" -> GoblinTokenTypes.KEYWORD;
            default -> GoblinTokenTypes.IDENTIFIER;
        };
    }

    private void locateOperator() {
        tokenEnd = tokenStart + 1;
        if (tokenEnd < endOffset) {
            String pair = buffer.subSequence(tokenStart, tokenEnd + 1).toString();
            if (pair.equals("<=") || pair.equals(">=") || pair.equals("!=") || pair.equals("==")) {
                tokenEnd++;
            }
        }
        tokenType = GoblinTokenTypes.OPERATOR;
    }

    private int untilNewline(int from) {
        int cursor = from;
        while (cursor < endOffset && buffer.charAt(cursor) != '\n' && buffer.charAt(cursor) != '\r') {
            cursor++;
        }
        return cursor;
    }

    private boolean startsWith(int offset, String value) {
        if (offset + value.length() > endOffset) {
            return false;
        }
        for (int index = 0; index < value.length(); index++) {
            if (buffer.charAt(offset + index) != value.charAt(index)) {
                return false;
            }
        }
        return true;
    }

    private boolean atIdentifierBoundary(int offset) {
        return offset >= endOffset || !isIdentifierContinue(buffer.charAt(offset));
    }

    private boolean isNumberStart(int offset) {
        char value = buffer.charAt(offset);
        return Character.isDigit(value)
                || (value == '.' && offset + 1 < endOffset && Character.isDigit(buffer.charAt(offset + 1)));
    }

    private static boolean isIdentifierStart(char value) {
        return Character.isLetter(value) || value == '_' || "πħωΩΔΣλμσθ∇∂".indexOf(value) >= 0;
    }

    private static boolean isIdentifierContinue(char value) {
        return isIdentifierStart(value) || Character.isDigit(value);
    }

    private static boolean isOperator(char value) {
        return "+-*/%^=<>!≤≥≠²³".indexOf(value) >= 0;
    }

    private static IElementType punctuationType(char value) {
        return switch (value) {
            case '(' -> GoblinTokenTypes.LEFT_PAREN;
            case ')' -> GoblinTokenTypes.RIGHT_PAREN;
            case '[' -> GoblinTokenTypes.LEFT_BRACKET;
            case ']' -> GoblinTokenTypes.RIGHT_BRACKET;
            case '{' -> GoblinTokenTypes.LEFT_BRACE;
            case '}' -> GoblinTokenTypes.RIGHT_BRACE;
            case ':', ',' -> GoblinTokenTypes.PUNCTUATION;
            default -> null;
        };
    }
}
