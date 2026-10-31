package org.goblinpp.jetbrains.actions;

import com.intellij.execution.ExecutionException;
import com.intellij.execution.configurations.GeneralCommandLine;
import com.intellij.execution.process.OSProcessHandler;
import com.intellij.execution.process.ProcessOutputTypes;
import com.intellij.openapi.application.ApplicationManager;
import com.intellij.openapi.project.Project;
import com.intellij.openapi.ui.Messages;
import com.intellij.openapi.util.Key;
import org.jetbrains.annotations.NotNull;
import java.nio.charset.StandardCharsets;
import java.nio.ByteBuffer;
import java.nio.charset.CodingErrorAction;
import java.util.Arrays;

/** Intercepts private input requests before console attachment; never echoes responses. */
public final class GoblinSecretInput extends OSProcessHandler {
    private static final String PREFIX="GOBLIN_SECRET_PROMPT_V1\t";
    private final Project project;
    private final StringBuilder pending=new StringBuilder();
    public GoblinSecretInput(Project project, GeneralCommandLine line) throws ExecutionException {
        super(line.withEnvironment("GOBLIN_SECRET_PROMPT_PROTOCOL","hex-v1"));
        this.project=project;
    }
    public static String decode(String line) {
        if (!line.startsWith(PREFIX)) return null;
        String hex=line.substring(PREFIX.length());
        if (hex.length()>8192 || hex.length()%2!=0 || !hex.matches("[0-9a-f]*")) return null;
        byte[] bytes=new byte[hex.length()/2];
        for(int i=0;i<bytes.length;i++) bytes[i]=(byte)Integer.parseInt(hex.substring(i*2,i*2+2),16);
        try {
            String prompt=StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString();
            return prompt.chars().anyMatch(Character::isISOControl)?null:prompt;
        } catch(java.nio.charset.CharacterCodingException ignored){return null;}
    }
    @Override public void notifyTextAvailable(@NotNull String text, @NotNull Key outputType) {
        if (!ProcessOutputTypes.STDERR.equals(outputType)) {super.notifyTextAvailable(text,outputType);return;}
        synchronized(pending) {
            pending.append(text);
            int newline;
            while((newline=pending.indexOf("\n"))>=0) {
                String line=pending.substring(0,newline).replaceFirst("\r$","");pending.delete(0,newline+1);
                if (line.startsWith(PREFIX)) {
                    String prompt=decode(line);
                    if(prompt==null){closeInput();continue;}
                    ApplicationManager.getApplication().invokeLater(()->{
                        if (project.isDisposed() || isProcessTerminated() || !com.intellij.ide.impl.TrustedProjects.isTrusted(project)) {closeInput();return;}
                        String answer=Messages.showPasswordDialog(project,prompt,"Goblin++ Secret Input",Messages.getQuestionIcon());
                        if(answer==null || answer.isEmpty() || answer.getBytes(StandardCharsets.UTF_8).length>4096 || answer.indexOf('\n')>=0 || answer.indexOf('\r')>=0 || answer.indexOf('\0')>=0){closeInput();return;}
                        byte[] payload=(answer+"\n").getBytes(StandardCharsets.UTF_8);
                        ApplicationManager.getApplication().executeOnPooledThread(()->{
                            try {var pipe=getProcessInput();if(pipe!=null){pipe.write(payload);pipe.flush();}}
                            catch(java.io.IOException ignored){closeInput();}
                            finally{Arrays.fill(payload,(byte)0);}
                        });
                    });
                } else {super.notifyTextAvailable(line+"\n",outputType);}
            }
            if(pending.length()>0 && !PREFIX.startsWith(pending.toString()) && !pending.toString().startsWith(PREFIX)) {
                super.notifyTextAvailable(pending.toString(),outputType);pending.setLength(0);
            }
            if(pending.length()>131072){super.notifyTextAvailable("Oversized program diagnostic omitted.\n",outputType);pending.setLength(0);closeInput();}
        }
    }
    private void closeInput(){try{var pipe=getProcessInput();if(pipe!=null)pipe.close();}catch(java.io.IOException ignored){}}
}
