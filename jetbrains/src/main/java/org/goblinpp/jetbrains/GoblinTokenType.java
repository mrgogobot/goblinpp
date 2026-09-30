package org.goblinpp.jetbrains;

import com.intellij.psi.tree.IElementType;
import org.jetbrains.annotations.NotNull;

public final class GoblinTokenType extends IElementType {
    public GoblinTokenType(@NotNull String debugName) {
        super(debugName, GoblinLanguage.INSTANCE);
    }

    @Override
    public String toString() {
        return "GoblinTokenType." + super.toString();
    }
}
