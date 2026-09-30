package org.goblinpp.jetbrains;

import com.intellij.lexer.Lexer;
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors;
import com.intellij.openapi.editor.HighlighterColors;
import com.intellij.openapi.editor.colors.TextAttributesKey;
import com.intellij.openapi.fileTypes.SyntaxHighlighterBase;
import com.intellij.psi.TokenType;
import com.intellij.psi.tree.IElementType;
import org.jetbrains.annotations.NotNull;

public final class GoblinSyntaxHighlighter extends SyntaxHighlighterBase {
    public static final TextAttributesKey COMMENT = key("GOBLIN_COMMENT", DefaultLanguageHighlighterColors.LINE_COMMENT);
    public static final TextAttributesKey STRING = key("GOBLIN_STRING", DefaultLanguageHighlighterColors.STRING);
    public static final TextAttributesKey NUMBER = key("GOBLIN_NUMBER", DefaultLanguageHighlighterColors.NUMBER);
    public static final TextAttributesKey DIRECTIVE = key("GOBLIN_DIRECTIVE", DefaultLanguageHighlighterColors.METADATA);
    public static final TextAttributesKey KEYWORD = key("GOBLIN_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD);
    public static final TextAttributesKey BOOLEAN = key("GOBLIN_BOOLEAN", DefaultLanguageHighlighterColors.KEYWORD);
    public static final TextAttributesKey FUNCTION = key("GOBLIN_FUNCTION", DefaultLanguageHighlighterColors.FUNCTION_CALL);
    public static final TextAttributesKey CONSTANT = key("GOBLIN_CONSTANT", DefaultLanguageHighlighterColors.CONSTANT);
    public static final TextAttributesKey UNIT = key("GOBLIN_UNIT", DefaultLanguageHighlighterColors.CLASS_NAME);
    public static final TextAttributesKey OPERATOR = key("GOBLIN_OPERATOR", DefaultLanguageHighlighterColors.OPERATION_SIGN);
    public static final TextAttributesKey INLINE_RUST = key("GOBLIN_INLINE_RUST", DefaultLanguageHighlighterColors.TEMPLATE_LANGUAGE_COLOR);
    public static final TextAttributesKey BAD = key("GOBLIN_BAD_CHARACTER", HighlighterColors.BAD_CHARACTER);

    private static TextAttributesKey key(String name, TextAttributesKey fallback) {
        return TextAttributesKey.createTextAttributesKey(name, fallback);
    }

    @Override
    public @NotNull Lexer getHighlightingLexer() {
        return new GoblinLexer();
    }

    @Override
    public TextAttributesKey @NotNull [] getTokenHighlights(IElementType tokenType) {
        if (tokenType == GoblinTokenTypes.COMMENT) return pack(COMMENT);
        if (tokenType == GoblinTokenTypes.STRING) return pack(STRING);
        if (tokenType == GoblinTokenTypes.NUMBER) return pack(NUMBER);
        if (tokenType == GoblinTokenTypes.DIRECTIVE) return pack(DIRECTIVE);
        if (tokenType == GoblinTokenTypes.KEYWORD) return pack(KEYWORD);
        if (tokenType == GoblinTokenTypes.BOOLEAN) return pack(BOOLEAN);
        if (tokenType == GoblinTokenTypes.FUNCTION) return pack(FUNCTION);
        if (tokenType == GoblinTokenTypes.CONSTANT) return pack(CONSTANT);
        if (tokenType == GoblinTokenTypes.UNIT) return pack(UNIT);
        if (tokenType == GoblinTokenTypes.OPERATOR) return pack(OPERATOR);
        if (tokenType == GoblinTokenTypes.INLINE_RUST) return pack(INLINE_RUST);
        if (tokenType == GoblinTokenTypes.LEFT_PAREN || tokenType == GoblinTokenTypes.RIGHT_PAREN
                || tokenType == GoblinTokenTypes.LEFT_BRACKET || tokenType == GoblinTokenTypes.RIGHT_BRACKET
                || tokenType == GoblinTokenTypes.LEFT_BRACE || tokenType == GoblinTokenTypes.RIGHT_BRACE) {
            return pack(DefaultLanguageHighlighterColors.BRACES);
        }
        if (tokenType == TokenType.BAD_CHARACTER) return pack(BAD);
        return TextAttributesKey.EMPTY_ARRAY;
    }
}
