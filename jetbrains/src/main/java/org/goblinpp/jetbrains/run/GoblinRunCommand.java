package org.goblinpp.jetbrains.run;

import com.intellij.util.execution.ParametersListUtil;

import java.util.ArrayList;
import java.util.List;

final class GoblinRunCommand {
    static List<String> build(String executable, String scriptPath, String argumentText) {
        List<String> command = new ArrayList<>();
        command.add(executable);
        command.add("run");
        command.add(scriptPath);
        List<String> programArguments = ParametersListUtil.parse(argumentText);
        if (!programArguments.isEmpty()) {
            command.add("--");
            command.addAll(programArguments);
        }
        return command;
    }

    private GoblinRunCommand() {
    }
}
