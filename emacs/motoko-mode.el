
;; Motoko major mode for Emacs
;; initially based on Swift Mode.

(setq motoko-font-lock-keywords
      (let* (
             ;; define several category of keywords
             ;; these are each taken from either Motoko's `lexer.mll' or `prelude.ml' files.
             (x-types
              '("Any"
                "None"
                "Shared"
                "Null"
                "Bool"
                "Nat"
                "Int"
                "Int8"
                "Int16"
                "Int32"
                "Int64"
                "Nat8"
                "Nat16"
                "Nat32"
                "Nat64"
                "Float"
                "Float32"
                "Char"
                "Text"
                "Blob"
                "Error"
                "Principal"
                "Region"))
              (x-constants
               '("null"
                 "true"
                 "false"
                 ))
              (x-keywords
               '("actor"
                 "and"
                 "async"
                 "async*"
                 "assert"
                 "await"
                 "await*"
                 "break"
                 "case"
                 "catch"
                 "class"
                 "continue"
                 "do"
                 "debug"
                 "debug_show"
                 "else"
                 "finally"
                 "flexible"
                 "for"
                 "from_candid"
                 "func"
                 "if"
                 "in"
                 "ignore"
                 "import"
                 "module"
                 "not"
                 "object"
                 "or"
                 "label"
                 "let"
                 "loop"
                 "persistent"
                 "private"
                 "public"
                 "query"
                 "return"
                 "shared"
                 "stable"
                 "switch"
                 "system"
                 "try"
                 "throw"
                 "to_candid"
                 "transient"
                 "with"
                 "type"
                 "var"
                 "while"
                 "prim"
                 "invariant"
                 "old"
                 "implies"
                 ))
              ;; Braces introduce blocks; it's nice to make them stand
              ;; out more than ordinary symbols
              (x-braces
               '( "{"
                  "}"))
              (x-symbols
               '( "("
                  ")"
                  "["
                  "]"
                  ;"{"
                  ;"}"
                  ";"
                  ","
                  ":"
                  "<:"
                  ;"\\."
                  ;"\\?"
                  "="
                  "<"
                  ">"
                  ;"\\+"
                  "-"
                  ;"\\*"
                  "/"
                  "%"
                  "**"
                  "&"
                  "|"
                  ;"\\^"
                  "<<"
                  ">>"
                  "<<>"
                  "<>>"
                  "#"
                  "=="
                  "!="
                  ">="
                  "<="
                  ":="
                  "+="
                  "-="
                  "*="
                  "/="
                  "%="
                  "**="
                  "&="
                  "|="
                  "^="
                  "<<="
                  ">>="
                  "<<>="
                  "<>>="
                  "#="
                  ))
              ;; xxx These still don't work:
              (x-symbols-more
               '( "\\."
                  "\\?"
                  "\\+"
                  "\\-"
                  "\\*"
                  "\\^"
                  ))
        ;; generate regex string for each category of keywords
        (x-types-regexp (regexp-opt x-types 'words))
        (x-constant-regexp (regexp-opt x-constants 'words))
        (x-keywords-regexp (regexp-opt x-keywords 'words))
        (x-braces-regexp (regexp-opt x-braces))
        (x-symbols-regexp (regexp-opt x-symbols))
        (x-symbols-more-regexp (regexp-opt x-symbols-more))
        )
        ;;
        `(
          (,x-types-regexp . font-lock-type-face)
          (,x-constant-regexp . font-lock-constant-face)
          (,x-keywords-regexp . font-lock-keyword-face)
          (,x-braces-regexp . font-lock-keyword-face)
          (,x-symbols-regexp . font-lock-builtin-face)
          (,x-symbols-more-regexp . font-lock-builtin-face)
          )))

(define-derived-mode motoko-mode
  swift-mode "Motoko"
  "Major mode for Motoko"
  (setq font-lock-defaults '((motoko-font-lock-keywords)))
  )

(add-to-list 'auto-mode-alist '("\\.mo\\'" . motoko-mode))

;; add the mode to the `features' list
(provide 'motoko-mode)
