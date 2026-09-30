package org.goblinpp.jetbrains;

import com.intellij.lang.Language;
import org.jetbrains.annotations.NotNull;

public final class GoblinLanguage extends Language {
    public static final GoblinLanguage INSTANCE = new GoblinLanguage();

    private GoblinLanguage() {
        super("Goblin++");
    }

    @Override
    public @NotNull String getDisplayName() {
        return "Goblin++";
    }
}
