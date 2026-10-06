package org.goblinpp.jetbrains;

import com.intellij.lexer.Lexer;
import com.intellij.psi.TokenType;
import com.intellij.psi.tree.IElementType;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

final class GoblinLexerTest {
    @Test
    void classifiesScientificProgramTokens() {
        String source = "GO_PARANOID\nlength = 3 m\ndiagonal = hypot(length, 4 m)\n"
                + "print(\"diagonal = {diagonal}\")\nseal diagonal\n";
        List<Token> tokens = lex(source);

        assertTrue(tokens.contains(new Token(GoblinTokenTypes.DIRECTIVE, "GO_PARANOID")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.FUNCTION, "hypot")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.UNIT, "m")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.STRING, "\"diagonal = {diagonal}\"")));
        assertFalse(tokens.stream().anyMatch(token -> token.type == TokenType.BAD_CHARACTER));
    }

    @Test
    void preservesInlineRustAsASeparateRegion() {
        String source = "RUST_INLINE_BEGIN\nprintln!(\"hello\");\nRUST_INLINE_END\nvalue = sqrt(9)\n";
        List<Token> tokens = lex(source);

        assertEquals(2, tokens.stream().filter(token -> token.type == GoblinTokenTypes.DIRECTIVE).count());
        assertTrue(tokens.stream().anyMatch(token -> token.type == GoblinTokenTypes.INLINE_RUST));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.FUNCTION, "sqrt")));
    }

    @Test
    void classifiesChemistryFunctionsConstantsAndUnicodeUnits() {
        String source = "mass = chem_molar_mass(\"H2O\")\nvolume = 25 µL\nline = 5 Å\nenergy = R * 300 K\n";
        List<Token> tokens = lex(source);

        assertTrue(tokens.contains(new Token(GoblinTokenTypes.FUNCTION, "chem_molar_mass")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.UNIT, "µL")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.UNIT, "Å")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.CONSTANT, "R")));
        assertFalse(tokens.stream().anyMatch(token -> token.type == TokenType.BAD_CHARACTER));
    }

    @Test
    void classifiesElectricalFunctionsAndUnicodeUnits() {
        List<Token> tokens = lex("resistance = 4.7 kΩ\ncapacitance = 220 µF\n"
                + "current = ee_current(12 V, resistance)\n"
                + "balanced = ee_kcl_balanced([3 mA, -3 mA], 0.01 mA)\n");

        assertTrue(tokens.contains(new Token(GoblinTokenTypes.FUNCTION, "ee_current")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.FUNCTION, "ee_kcl_balanced")));
        for (String unit : List.of("kΩ", "µF", "V", "mA")) {
            assertTrue(tokens.contains(new Token(GoblinTokenTypes.UNIT, unit)));
        }
        assertFalse(tokens.stream().anyMatch(token -> token.type == TokenType.BAD_CHARACTER));
    }

    private static List<Token> lex(String source) {
        Lexer lexer = new GoblinLexer();
        lexer.start(source);
        List<Token> tokens = new ArrayList<>();
        while (lexer.getTokenType() != null) {
            tokens.add(new Token(
                    lexer.getTokenType(),
                    source.substring(lexer.getTokenStart(), lexer.getTokenEnd())));
            lexer.advance();
        }
        return tokens;
    }

    @Test
    void classifiesCompoundUnitSuffixes() {
        List<Token> tokens = lex("a = 1.13e-10 m/s^2\narea = 3 m²\np = 7 kg/(m*s^2)\n");
        for (String unit : List.of("m", "s", "kg")) {
            assertTrue(tokens.contains(new Token(GoblinTokenTypes.UNIT, unit)));
        }
        assertFalse(tokens.stream().anyMatch(token -> token.type == TokenType.BAD_CHARACTER));
    }

    @Test
    void classifiesLocalImportsAndTableReaders() {
        List<Token> tokens = lex("import \"lib/measurements.gbl\"\nvalues = csv_numbers(\"data.csv\", \"length\")\nnames = tsv_column(\"data.tsv\", \"sample\")\n");
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.KEYWORD, "import")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.FUNCTION, "csv_numbers")));
        assertTrue(tokens.contains(new Token(GoblinTokenTypes.FUNCTION, "tsv_column")));
        assertFalse(tokens.stream().anyMatch(token -> token.type == TokenType.BAD_CHARACTER));
    }

    private record Token(IElementType type, String text) {
    }
}
