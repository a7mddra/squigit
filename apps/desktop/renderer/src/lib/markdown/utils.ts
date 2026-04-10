export function preprocessMarkdown(
  markdown: string,
  options: { doubleNewlines?: boolean } = {},
): string {
  const lines = markdown.split("\n");
  const resultLines: string[] = [];

  let codeBlockDepth = 0;
  let inCodeBlock = false;
  let codeBlockFence = "";
  let codeBlockLang = "";
  let codeBlockContent: string[] = [];
  let textBuffer: string[] = [];

  const flushTextBuffer = () => {
    if (textBuffer.length > 0) {
      let text = textBuffer.join("\n");
      if (options.doubleNewlines) {
        text = text.replace(/\n/g, "\n\n");
      }
      resultLines.push(text);
      textBuffer = [];
    }
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    const fenceMatch = line.match(/^(\s*)(`{3,})(.*)$/);

    if (fenceMatch) {
      const indent = fenceMatch[1];
      const fence = fenceMatch[2];
      const lang = fenceMatch[3].trim();

      if (!inCodeBlock) {
        flushTextBuffer();
        inCodeBlock = true;
        codeBlockDepth = 1;
        codeBlockFence = fence;
        codeBlockLang = lang;
        codeBlockContent = [];
      } else {
        if (lang !== "") {
          codeBlockDepth++;
          codeBlockContent.push(line);
        } else {
          if (fence.length >= codeBlockFence.length) {
            codeBlockDepth--;
          } else {
            codeBlockContent.push(line);
            continue;
          }

          if (codeBlockDepth === 0) {
            const content = codeBlockContent.join("\n");

            const innerFences = content.match(/`{3,}/g);
            let maxFenceLength = 0;
            if (innerFences) {
              maxFenceLength = Math.max(...innerFences.map((f) => f.length));
            }

            let outerFence = codeBlockFence;
            if (maxFenceLength >= outerFence.length) {
              outerFence = "`".repeat(maxFenceLength + 1);
            }

            resultLines.push(`${indent}${outerFence}${codeBlockLang}`);
            resultLines.push(content);
            resultLines.push(`${indent}${outerFence}`);

            inCodeBlock = false;
            codeBlockFence = "";
            codeBlockLang = "";
            codeBlockDepth = 0;
            codeBlockContent = [];
          } else {
            codeBlockContent.push(line);
          }
        }
      }
    } else {
      if (inCodeBlock) {
        codeBlockContent.push(line);
      } else {
        textBuffer.push(line);
      }
    }
  }

  if (inCodeBlock) {
    const content = codeBlockContent.join("\n");
    const innerFences = content.match(/`{3,}/g);
    let maxFenceLength = 0;
    if (innerFences) {
      maxFenceLength = Math.max(...innerFences.map((f) => f.length));
    }
    let outerFence = codeBlockFence;
    if (maxFenceLength >= outerFence.length) {
      outerFence = "`".repeat(maxFenceLength + 1);
    }

    resultLines.push(`${outerFence}${codeBlockLang}`);
    resultLines.push(content);
    resultLines.push(`${outerFence}`);
  } else {
    if (textBuffer.length > 0) {
      let text = textBuffer.join("\n");
      if (options.doubleNewlines) {
        text = text.replace(/\n/g, "\n\n");
      }
      resultLines.push(text);
    }
  }

  return resultLines.join("\n");
}

export function splitMarkdownAfterLastClosedFence(markdown: string): {
  stable: string;
  tail: string;
} {
  if (!markdown) {
    return {
      stable: "",
      tail: "",
    };
  }

  const lines = markdown.split("\n");
  let inFence = false;
  let fenceChar = "";
  let fenceLength = 0;
  let offset = 0;
  let lastClosedFenceEndOffset = 0;

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const fenceMatch = line.match(/^\s*(`{3,}|~{3,})(.*)$/u);
    const lineTerminatorLength = i < lines.length - 1 ? 1 : 0;

    if (fenceMatch) {
      const marker = fenceMatch[1];
      const suffix = fenceMatch[2];

      if (!inFence) {
        inFence = true;
        fenceChar = marker[0];
        fenceLength = marker.length;
      } else if (
        marker[0] === fenceChar &&
        marker.length >= fenceLength &&
        suffix.trim().length === 0
      ) {
        inFence = false;
        fenceChar = "";
        fenceLength = 0;
        lastClosedFenceEndOffset = offset + line.length + lineTerminatorLength;
      }
    }

    offset += line.length + lineTerminatorLength;
  }

  if (
    lastClosedFenceEndOffset <= 0 ||
    lastClosedFenceEndOffset >= markdown.length
  ) {
    return {
      stable: markdown,
      tail: "",
    };
  }

  return {
    stable: markdown.slice(0, lastClosedFenceEndOffset),
    tail: markdown.slice(lastClosedFenceEndOffset),
  };
}
