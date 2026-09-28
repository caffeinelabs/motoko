;; -*- lexical-binding: t; -*-

(defun motoko-ts-font-lock-rules ()
  (treesit-font-lock-rules
   :language 'motoko
   :override t
   :feature 'delimiter
   '((["(" ")" "[" "]" "{" "}" "<" ">"] @font-lock-bracket-face)
     ([";" ","] @font-lock-delimiter-face))

   :language 'motoko
   :override t
   :feature 'comment
   '(([(line_comment) (block_comment)] @font-lock-comment-face)
     ((doc_comment) @font-lock-doc-face))

   :language 'motoko
   :override t
   :feature 'constant
   '(([(text_literal) (char_literal)] @font-lock-string-face)
     ([(int_literal) (float_literal) (hex_literal)] @font-lock-number-face)
     ([(null_literal) (bool_literal)] @font-lock-constant-face))

   :language 'motoko
   :override t
   :feature 'keyword
   '(["actor" "and" "await" "await*" "break" "case" "catch"
      "class" "continue" "composite" "debug" "do"
      "else" "finally" "for" "func" "if" "in" "import"
      "module" "not" "object" "or" "label" "let" "loop"
      "private" "public" "return" "shared"
      "try" "throw" "query" "switch" "type"
      "var" "while" "with" "stable" "system"
      "assert" "ignore" "async" "async*" "persistent" "transient"]
     @font-lock-keyword-face)

   :language 'motoko
   :override t
   :feature 'delimiter
   '([(unop) (unassign_op) (binassign_op) (rel_op) (bin_op)] @font-lock-operator-face)

   :language 'motoko
   :override t
   :feature 'calls
   '((call_exp_object (dot_exp_object (identifier) @font-lock-function-call-face))
     (call_exp_object (var_exp (identifier) @font-lock-function-call-face))
     (call_exp_block (dot_exp_block (identifier) @font-lock-function-call-face))
     (call_exp_block (var_exp (identifier) @font-lock-function-call-face))
     (tag_identifier) @font-lock-function-call-face)

   :language 'motoko
   :override t
   :feature 'calls
   '((exp_field (identifier) @font-lock-property-use-face)
     (func_tf (identifier) @font-lock-property-name-face)
     (val_tf (identifier) @font-lock-property-name-face))

   :language 'motoko
   :override t
   :feature 'keyword
   '((type_identifier) @font-lock-type-face)

   :language 'motoko
   :override t
   :feature 'declaration
   '((func_dec name: (identifier) @font-lock-function-name-face))
   ))


(defvar motoko-ts-indent 2)

;; For debugging
; (setq treesit--indent-verbose t)
(defvar motoko-ts-indent-rules
  `((motoko
     ((parent-is "source_file") column-0 0)
     ((node-is ")") parent-bol 0)
     ((node-is "]") parent-bol 0)
     ((node-is "}") (and parent parent-bol) 0)
     ((parent-is "func_dec") parent-bol motoko-ts-indent)
     ((parent-is "obj_body") parent-bol motoko-ts-indent)
     ((parent-is "object_exp") parent-bol motoko-ts-indent)
     ((parent-is "block_exp") parent-bol motoko-ts-indent)
     ((parent-is "par_exp") parent-bol motoko-ts-indent)
     ((parent-is "obj_typ") parent-bol motoko-ts-indent)
     ((parent-is "dec_obj") parent-bol motoko-ts-indent)
     ((parent-is "let_dec") parent-bol motoko-ts-indent)
     ((parent-is "let_else_dec") parent-bol motoko-ts-indent)
     ((parent-is "var_dec") parent-bol motoko-ts-indent)
     ((parent-is "switch_exp") parent-bol motoko-ts-indent)
     ((parent-is "tup_pat") parent-bol motoko-ts-indent)
     )))

(defun motoko-ts-setup ()
  "Setup treesit for motoko-ts-mode."

  (setq-local treesit-font-lock-feature-list
              '((comment delimiter)
                (constant keyword calls)
                (declaration)))

  (setq-local treesit-font-lock-settings (motoko-ts-font-lock-rules))

  (setq-local treesit-simple-indent-rules motoko-ts-indent-rules)

  (treesit-major-mode-setup))

(define-derived-mode motoko-ts-mode prog-mode "Motoko"
  "Major mode for editing Motoko source files"
  (when (treesit-ready-p 'motoko)
    (treesit-parser-create 'motoko)
    (motoko-ts-setup)))

(add-to-list 'auto-mode-alist '("\\.mo\\'" . motoko-ts-mode))

(when 'treesit-language-source-alist
  (add-to-list 'treesit-language-source-alist '(motoko "https://github.com/christoph-dfinity/tree-sitter-motoko")))

(provide 'motoko-ts-mode)
