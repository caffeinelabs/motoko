import React, { useState, useEffect, useCallback, useRef } from "react";
import ExecutionEnvironment from "@docusaurus/ExecutionEnvironment";
import String from "@theme-original/CodeBlock/Content/String";
import Container from "@theme/CodeBlock/Container";
import styles from "./styles.module.css";
import hljs from "highlight.js";
import { extractConfig, handleRun } from "../hljs_run.js";
import runIcon from "@site/static/img/runIcon.png";

function SimpleCopyButton({ className, code }) {
  const [isCopied, setIsCopied] = useState(false);
  const timeoutRef = useRef(undefined);

  useEffect(() => () => clearTimeout(timeoutRef.current), []);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(code).then(() => {
      setIsCopied(true);
      timeoutRef.current = setTimeout(() => setIsCopied(false), 1000);
    });
  }, [code]);

  return (
    <button
      type="button"
      className={className}
      aria-label={isCopied ? "Copied" : "Copy code to clipboard"}
      title="Copy"
      onClick={handleCopy}
    >
      {isCopied ? "Copied" : "Copy"}
    </button>
  );
}

// NOTE: String component of CodeBlock is being swizzled as a wrapped component.

function RunButton(props) {
  // buttons with class "run" will be run in load_moc.js when moc is loaded.
  const className = props.config.isRun ? "run-button run" : "run-button";
  return (
    <button
      type="button"
      className={className}
      aria-label="Run"
      onClick={() => handleRun(props)}
      title="Run Code"
    >
      <img src={runIcon} style={{ width: "20px", height: "20px" }} />
    </button>
  );
}

function ImmutableCodeBlock({ id, code, language, defaultCopy }) {
  const ref = React.createRef();
  useEffect(() => {
    hljs.highlightElement(ref.current);
  }, []);
  return (
    <Container as="div" className={styles.immutableCodeBlock}>
      <pre id={id} className={language} ref={ref}>
        <code>{code}</code>
      </pre>
      {/* defaultCopy is a flag for code (candid) with only copy button */}
      {defaultCopy && (
        <div className={styles.buttonGroup}>
          <SimpleCopyButton className={styles.copyButton} code={code} />
        </div>
      )}
    </Container>
  );
}

export default function StringWrapper(props) {
  if (props.className === "language-motoko" && ExecutionEnvironment.canUseDOM) {

    // color no-repl blocks pink but allow running (for review)
    const backColor =
      (!props.hasOwnProperty("no-repl"))?
        "var(--prism-background-color)" :
        "pink";

    const { useCodeJar } = require('react-codejar');
    const [code, setCode] = useState(props.children || "");
    const [output, setOutput] = useState("");
    const [error, setError] = useState("");
    const lineNumbers = props.children.split("\n").length > 3;

    // syntax highlighting is done by CodeJar, creating new React components
    const editorRef = useCodeJar({
      code: code.replace(/^\s+|\s+$/g, ''), // trim newlines
      onUpdate: (e) => {
        setCode(e);
      },
      highlight: hljs.highlightElement,
      lineNumbers,
    });
    return (
      <>
        <Container as="div">
          <div className={styles.codeBlockContent}>
            <pre
              id={props.name}
              ref={editorRef}
              className="language-motoko"
              style={{ backgroundColor: backColor }}
            >
              <code>{code}</code>
            </pre>
            <div className={styles.buttonGroup}>
              <SimpleCopyButton className={styles.copyButton} code={code} />
              <RunButton
                code={code}
                setOutput={setOutput}
                setError={setError}
                config={extractConfig(props)}
              />
            </div>
          </div>
        </Container>
        {output || error ? (
          <Container as="div">
            {error ? <pre className="motoko-code-error">{error}</pre> : null}
            {output ? (
              <pre className="motoko-code-output">
                <code>{output}</code>
              </pre>
            ) : null}
          </Container>
        ) : null}
      </>
    );
  }
  if (props.className === "language-candid" && ExecutionEnvironment.canUseDOM) {
    // for candid code no run button is given
    return (
      <>
        <ImmutableCodeBlock
          code={props.children}
          language="language-candid"
          style={{ position: "relative" }}
          defaultCopy={true}
        />
      </>
    );
  }
  // default Docusaurus built-in String wrapper, leave as is
  return (
    <>
      <String {...props} />
    </>
  );
}
