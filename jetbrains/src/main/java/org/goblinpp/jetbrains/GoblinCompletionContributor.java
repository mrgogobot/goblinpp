package org.goblinpp.jetbrains;

import com.intellij.codeInsight.completion.CompletionContributor;
import com.intellij.codeInsight.completion.CompletionParameters;
import com.intellij.codeInsight.completion.CompletionProvider;
import com.intellij.codeInsight.completion.CompletionResultSet;
import com.intellij.codeInsight.completion.CompletionType;
import com.intellij.codeInsight.lookup.LookupElementBuilder;
import com.intellij.patterns.PlatformPatterns;
import com.intellij.util.ProcessingContext;
import org.jetbrains.annotations.NotNull;

public final class GoblinCompletionContributor extends CompletionContributor {
    public GoblinCompletionContributor() {
        extend(
                CompletionType.BASIC,
                PlatformPatterns.psiElement().withLanguage(GoblinLanguage.INSTANCE),
                new CompletionProvider<>() {
                    @Override
                    protected void addCompletions(
                            @NotNull CompletionParameters parameters,
                            @NotNull ProcessingContext context,
                            @NotNull CompletionResultSet result) {
                        for (GoblinVocabulary.Entry entry : GoblinVocabulary.entries()) {
                            LookupElementBuilder item = LookupElementBuilder
                                    .create(entry.spelling())
                                    .withTypeText(entry.kind(), true)
                                    .withTailText("  " + entry.detail(), true);
                            if ("function".equals(entry.kind())) {
                                item = item.withInsertHandler((insertion, ignored) -> {
                                    int tail = insertion.getTailOffset();
                                    CharSequence text = insertion.getDocument().getCharsSequence();
                                    if (tail >= text.length() || text.charAt(tail) != '(') {
                                        insertion.getDocument().insertString(tail, "()");
                                        insertion.getEditor().getCaretModel().moveToOffset(tail + 1);
                                    }
                                });
                            }
                            result.addElement(item);
                        }
                    }
                });
    }
}
