"Primitive macro-assembler for WAT files.

This is based on a trick I learned from MIT's Rod Brooks. It's a
macro assembler that uses s-expressions and Lisp-like macros. In
this case, the output is a WebAssembly WAT file.

Note that this approach really only works where Common Lisp and
WAT have mostly similar syntax. This assembler is intended to
be used on a limited set of known files to bootstrap a Lisp runtime,
so some rough edges are acceptable."

(defun splice-reader (stream char)
  "Reader macro to make [...] into (splice ...)."
  (declare (ignore char))
  (let ((contents (read-delimited-list #\] stream t)))
    `(|splice| ,@contents)))

(set-macro-character #\[ #'splice-reader)
(set-macro-character #\] (get-macro-character #\)))

(defvar *wat-readtable* (copy-readtable)
  "Read-table for #w(...) expresions.")
  (setf (readtable-case *wat-readtable*) :preserve)

(defun wat-reader (stream subchar arg)
  "Read an s-expression preserving case."
  (declare (ignore subchar) (ignore arg))
  (let ((*readtable* *wat-readtable*))
    (read stream t nil t)))

(set-dispatch-macro-character #\# #\w #'wat-reader)

(defun read-wat-file (path)
  "Read PATH preserving case."
  (with-open-file (stream path)
    (let ((*readtable* *wat-readtable*))
      (read stream))))

(defun perform-splices (value)
  "Recursively flatten (... (splice ...) ...) forms in s-expressions.

  This is used after macroexpression in WAT lists, because we often
  need to flatten multiple assembly instructions into the body of a
  function."
  (cond
    ((and
       (consp value)
       (consp (car value))
       (eq (caar value) '|splice|))
     (concatenate 'list
       (perform-splices (cdar value))
       (perform-splices (cdr value))))
    ((consp value)
     (cons (perform-splices (car value)) (perform-splices (cdr value))))
    (t value)))

(defvar *wat-macros* (make-hash-table)
  "Hash table of macros appearing in WAT files.")

(defmacro define-wat-macro (name args &body body)
  "Define a macro which can be used in a WAT file."
  `(setf (gethash ',name *wat-macros*)
         #'(lambda ,args
             ,@body)))

(defun macroexpand-1-wat (form)
  "If (car FORM) is a WAT macro, expand it."
  (if (consp form)
      (let ((macro (gethash (car form) *wat-macros*)))
        (if macro
            (apply macro (cdr form))
            form))
      form))

(defun macroexpand-wat (form)
  "As long as (car FORM) is a WAT macro, keep expanding it."
  (let ((expanded (macroexpand-1-wat form)))
    (if (equal expanded form)
        expanded
        (macroexpand-wat expanded))))

(defun recursive-macroexpand-wat (form)
  "Walk FORM recursively, calling MACROEXPAND-WAT to expand any
  WAT macros."
  (if (listp form)
    (map 'list #'recursive-macroexpand-wat (macroexpand-wat form))
    form))

(defun assemble-wat (form)
  "Given FORM, expand WAT macros and splice the output, returning a
  WAT form that can be printed with PRINT-WAT."
  (perform-splices (recursive-macroexpand-wat form)))

(defun assemble-wat-file (path)
  "Read PATH using READ-WAT-FILE, expand macros and splice the output."
  (let ((form (read-wat-file path)))
    (assemble-wat form)))

(defun print-wat (form &optional (stream nil))
  "Print FORM as WAT, preserving case."
  (let ((*readtable* *wat-readtable*)
        (*print-escape* t)
        (*print-pretty* t))
    (print form stream)))

(defun print-wat-file (form path)
  "Print FORM to PATH, preserving case."
  (with-open-file (stream path
                   :if-does-not-exist :create
                   :if-exists :overwrite
                   :direction :output)
    (print-wat form stream)))

(define-wat-macro |%call| (FUNC &rest ARGS)
  "Push ARGS and call FUNC."
  #w`[,@ARGS
      call ,FUNC])

(define-wat-macro |%cons| (CAR-EXPR CDR-EXPR)
  "Build a cons cell."
  #w`(%call $prim_mkcons ,CAR-EXPR ,CDR-EXPR))

(define-wat-macro |%include| (path)
  "Split the contents of PATH at the current location.

  Strips the '(module name ...) wrapper. NAME is assumed to match the
  current file, but this is not checked."
  (let* ((form (read-wat-file path))
         (BODY (cddr form)))
    #w`[,@BODY]))

(defun build-runtime ()
  "Build our toy Lisp runtime as WAT and WASM."
  (let ((wat (assemble-wat-file "runtime/runtime.watm")))
    (print-wat-file wat "runtime/runtime.wat")
    ; (sb-ext:run-program
    ;   "wasm-tools"
    ;   '("parse" "runtime/runtime.wat" "-o" "runtime/runtime.wasm")
    ;   :output *standard-output*)
    ))
(build-runtime)

#|
(print-wat
  (assemble-wat
    #w'(func $example (type $example_type)
          (%cons [global.get $nil] [global.get $nil])
          drop)))

(print-wat (assemble-wat-file "runtime/runtime.watm"))
|#
