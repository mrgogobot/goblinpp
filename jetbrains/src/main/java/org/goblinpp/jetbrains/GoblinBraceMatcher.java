package org.goblinpp.jetbrains;

import com.intellij.lang.BracePair;
import com.intellij.lang.PairedBraceMatcher;
import com.intellij.psi.PsiFile;
import com.intellij.psi.tree.IElementType;
import org.jetbrains.annotations.NotNull;

public final class GoblinBraceMatcher implements PairedBraceMatcher {
    private static final BracePair[] PAIRS = {
            new BracePair(GoblinTokenTypes.LEFT_PAREN, GoblinTokenTypes.RIGHT_PAREN, false),
            new BracePair(GoblinTokenTypes.LEFT_BRACKET, GoblinTokenTypes.RIGHT_BRACKET, false),
            new BracePair(GoblinTokenTypes.LEFT_BRACE, GoblinTokenTypes.RIGHT_BRACE, true)
    };

    @Override
    public BracePair @NotNull [] getPairs() {
        return PAIRS;
    }

    @Override
    public boolean isPairedBracesAllowedBeforeType(@NotNull IElementType leftBraceType, IElementType contextType) {
        return true;
    }

    @Override
    public int getCodeConstructStart(PsiFile file, int openingBraceOffset) {
        return openingBraceOffset;
    }
}
