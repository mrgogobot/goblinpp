package org.goblinpp.jetbrains;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class GoblinVocabulary {
    public record Entry(String spelling, String kind, String detail, String snippet) {
    }

    private static final List<Entry> ENTRIES = load();
    private static final Map<String, Entry> BY_SPELLING = index(ENTRIES);

    public static List<Entry> entries() {
        return ENTRIES;
    }

    public static Entry find(String spelling) {
        return BY_SPELLING.get(spelling);
    }

    public static String kindOf(String spelling) {
        Entry entry = find(spelling);
        return entry == null ? null : entry.kind();
    }

    private static List<Entry> load() {
        InputStream stream = GoblinVocabulary.class.getClassLoader()
                .getResourceAsStream("goblinpp-vocabulary.tsv");
        if (stream == null) {
            throw new IllegalStateException("Goblin++ vocabulary resource is missing");
        }
        Base64.Decoder decoder = Base64.getDecoder();
        List<Entry> entries = new ArrayList<>();
        try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(stream, StandardCharsets.UTF_8))) {
            String line;
            while ((line = reader.readLine()) != null) {
                String[] fields = line.split("\\t", -1);
                if (fields.length != 4) {
                    throw new IllegalStateException("Malformed Goblin++ vocabulary row");
                }
                entries.add(new Entry(
                        decode(decoder, fields[0]),
                        decode(decoder, fields[1]),
                        decode(decoder, fields[2]),
                        decode(decoder, fields[3])));
            }
        } catch (IOException | IllegalArgumentException error) {
            throw new IllegalStateException("Unable to load Goblin++ vocabulary", error);
        }
        return Collections.unmodifiableList(entries);
    }

    private static String decode(Base64.Decoder decoder, String value) {
        return new String(decoder.decode(value), StandardCharsets.UTF_8);
    }

    private static Map<String, Entry> index(List<Entry> entries) {
        Map<String, Entry> result = new LinkedHashMap<>();
        for (Entry entry : entries) {
            result.put(entry.spelling(), entry);
        }
        return Collections.unmodifiableMap(result);
    }

    private GoblinVocabulary() {
    }
}
