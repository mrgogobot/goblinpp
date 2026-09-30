package org.goblinpp.jetbrains;

import com.intellij.openapi.fileTypes.LanguageFileType;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import javax.swing.Icon;

public final class GoblinFileType extends LanguageFileType {
    public static final GoblinFileType INSTANCE = new GoblinFileType();

    private GoblinFileType() {
        super(GoblinLanguage.INSTANCE);
    }

    @Override
    public @NotNull String getName() {
        return "Goblin++";
    }

    @Override
    public @NotNull String getDescription() {
        return "Goblin++ scientific program";
    }

    @Override
    public @NotNull String getDefaultExtension() {
        return "gbl";
    }

    @Override
    public @Nullable Icon getIcon() {
        return GoblinIcons.FILE;
    }
}
