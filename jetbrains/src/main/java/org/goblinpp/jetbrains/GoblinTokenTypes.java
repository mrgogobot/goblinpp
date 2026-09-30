package org.goblinpp.jetbrains;

import com.intellij.psi.TokenType;
import com.intellij.psi.tree.IElementType;
import com.intellij.psi.tree.TokenSet;

public final class GoblinTokenTypes {
    public static final IElementType COMMENT = new GoblinTokenType("COMMENT");
    public static final IElementType STRING = new GoblinTokenType("STRING");
    public static final IElementType NUMBER = new GoblinTokenType("NUMBER");
    public static final IElementType DIRECTIVE = new GoblinTokenType("DIRECTIVE");
    public static final IElementType KEYWORD = new GoblinTokenType("KEYWORD");
    public static final IElementType BOOLEAN = new GoblinTokenType("BOOLEAN");
    public static final IElementType FUNCTION = new GoblinTokenType("FUNCTION");
    public static final IElementType CONSTANT = new GoblinTokenType("CONSTANT");
    public static final IElementType UNIT = new GoblinTokenType("UNIT");
    public static final IElementType IDENTIFIER = new GoblinTokenType("IDENTIFIER");
    public static final IElementType OPERATOR = new GoblinTokenType("OPERATOR");
    public static final IElementType LEFT_PAREN = new GoblinTokenType("LEFT_PAREN");
    public static final IElementType RIGHT_PAREN = new GoblinTokenType("RIGHT_PAREN");
    public static final IElementType LEFT_BRACKET = new GoblinTokenType("LEFT_BRACKET");
    public static final IElementType RIGHT_BRACKET = new GoblinTokenType("RIGHT_BRACKET");
    public static final IElementType LEFT_BRACE = new GoblinTokenType("LEFT_BRACE");
    public static final IElementType RIGHT_BRACE = new GoblinTokenType("RIGHT_BRACE");
    public static final IElementType PUNCTUATION = new GoblinTokenType("PUNCTUATION");
    public static final IElementType INLINE_RUST = new GoblinTokenType("INLINE_RUST");
    public static final IElementType BAD_CHARACTER = TokenType.BAD_CHARACTER;

    public static final TokenSet COMMENTS = TokenSet.create(COMMENT);
    public static final TokenSet STRINGS = TokenSet.create(STRING);

    private GoblinTokenTypes() {
    }
}
