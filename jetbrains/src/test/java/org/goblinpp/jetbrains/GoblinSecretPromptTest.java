package org.goblinpp.jetbrains;
import org.goblinpp.jetbrains.actions.GoblinSecretInput;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;
final class GoblinSecretPromptTest {
    @Test void validPromptDecodesAndMalformedControlAndUtf8Refuse(){
        assertEquals("Passphrase: ",GoblinSecretInput.decode("GOBLIN_SECRET_PROMPT_V1\t506173737068726173653a20"));
        for(String value:new String[]{"ff","0a","abc","00","61".repeat(4097)})assertNull(GoblinSecretInput.decode("GOBLIN_SECRET_PROMPT_V1\t"+value));
        assertNull(GoblinSecretInput.decode("ordinary output"));
    }
    @Test void privateFunctionVocabularyIsAvailable(){
        for(String name:new String[]{"GO_PRIVATE","input_secret","encrypt_text","encrypt_file","decrypt_text","decrypt_file","read_text","display_file"})assertNotNull(GoblinVocabulary.find(name),name);
    }
}
