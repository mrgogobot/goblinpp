package org.goblinpp.jetbrains;

import com.intellij.lang.documentation.AbstractDocumentationProvider;
import com.intellij.openapi.editor.Editor;
import com.intellij.psi.PsiElement;
import com.intellij.psi.PsiFile;
import com.intellij.psi.util.PsiTreeUtil;
import org.jetbrains.annotations.Nullable;

public final class GoblinDocumentationProvider extends AbstractDocumentationProvider {
    @Override
    public @Nullable PsiElement getCustomDocumentationElement(
            Editor editor,
            PsiFile file,
            PsiElement contextElement,
            int targetOffset) {
        PsiElement leaf = file.findElementAt(targetOffset);
        return leaf != null && GoblinVocabulary.find(leaf.getText()) != null ? leaf : null;
    }

    @Override
    public @Nullable String generateDoc(PsiElement element, @Nullable PsiElement originalElement) {
        PsiElement leaf = element.getFirstChild() == null ? element : PsiTreeUtil.getDeepestFirst(element);
        GoblinVocabulary.Entry entry = GoblinVocabulary.find(leaf.getText());
        if (entry == null) {
            return null;
        }
        String snippet = entry.snippet().isBlank()
                ? ""
                : "<p><b>Example</b></p><pre>" + escape(entry.snippet()) + "</pre>";
        return "<div class='definition'><pre>" + escape(entry.spelling()) + "</pre></div>"
                + "<div class='content'><p><b>" + escape(entry.kind()) + "</b></p><p>"
                + escape(entry.detail()) + "</p>" + snippet + "</div>";
    }

    private static String escape(String value) {
        return value.replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("\"", "&quot;");
    }
}
