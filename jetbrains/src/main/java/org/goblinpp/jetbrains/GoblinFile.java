package org.goblinpp.jetbrains;

import com.intellij.extapi.psi.PsiFileBase;
import com.intellij.psi.FileViewProvider;
import org.jetbrains.annotations.NotNull;

public final class GoblinFile extends PsiFileBase {
    public GoblinFile(@NotNull FileViewProvider viewProvider) {
        super(viewProvider, GoblinLanguage.INSTANCE);
    }

    @Override
    public @NotNull GoblinFileType getFileType() {
        return GoblinFileType.INSTANCE;
    }

    @Override
    public String toString() {
        return "Goblin++ File";
    }
}
