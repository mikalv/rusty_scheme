;;(use-modules (sfri sfri-9))

;;;; bytecode.scm – bytecode objects and bytecode generation
;;;
;;; This library defines the type of bytecode objects.  It also defines
;;; operations for appending bytecodes and a DSL for assembling them.

(library
    (bytecode)
  (export emit-bindings
          emit-variable-reference
          emit-lambda-definition
          add-to-constant-vector
          len
          instrs
          consts
          stack-depth
          memo
          create-bco
          emit-apply
          emit-jump
          emit-constant
          emit-set!
          emit-primitive
          emit-global)
  (import
   (rnrs)
   (only (srfi :1) proper-list?)
   (srfi :9)
   (srfi :43)
   (srfi :69))

  (define-record-type :bco
    (make-bco len instrs consts consts-len stack-depth memo counter)
    bco?
    (len len len-set!)
    (instrs instrs instrs-set!)
    (consts consts consts-set!)
    (consts-len consts-len consts-len-set!)
    (stack-depth stack-depth stack-depth-set!)
    (memo memo)
    (counter counter counter-set!))

  #;(define-record-type
  (bco make-bco bco?)
  (fields
  (mutable len len len-set!)
  (mutable instrs instrs instrs-set!)
  (mutable consts consts consts-set!)
  (mutable consts-len consts-len consts-len-set!)
  (mutable stack-depth stack-depth stack-depth-set!)
  (sealed #t)
  (opaque #t)))
  (define (create-bco)
    (make-bco 0 #() #(#f) 0 0 (make-hash-table) 0))

  (define-syntax add-bytecodes
    (syntax-rules ()
      ((_ bytecode-object operations more-operations ...)
       (begin
         (add-bytecode bytecode-object operations)
         (add-bytecode bytecode-object more-operations) ...))))

  ;; Add `object` to the constant vector of `bco`.
  ;; Returns the index of `object` in the constant vector of `bco`.
  ;; `object` must not be modified afterwords.
  (define (add-to-constant-vector bco object)
    (assert (not (procedure? object)))
    (let ((constants (consts bco))
          (bco-consts-len (consts-len bco)))
      (assert (fixnum? bco-consts-len))
      (let ((capacity (vector-length constants)))
        (vector-set!
         (if (= bco-consts-len capacity)
             (let ((new-vector
                    (vector-copy constants
                                 0
                                 (+ 3 (* 2 capacity)) #f)))
               (consts-set! bco new-vector)
               new-vector)
             constants)
         bco-consts-len object)
        (consts-len-set! bco (+ 1 bco-consts-len))
        bco-consts-len)))

  (define (emit bco . opcode)
    (assert (bco? bco))
    (let ((bytecode (instrs bco))
          (bco-len (len bco)))
      (let ((capacity (vector-length bytecode)))
        (vector-set!
         (cond
          ((= bco-len capacity)
           (let ((new-vector (vector-copy bytecode 0 (+ 3 (* 2 bco-len)) #f)))
             (instrs-set! bco new-vector)
             new-vector))
          ((< bco-len capacity)
           bytecode)
          (else
           (assert #f)))
         bco-len opcode)
        (len-set! bco (+ 1 bco-len))
        bco-len)))

  (define (emit-global bco symbol)
    (emit bco 'global-load symbol)
    (let ((new-depth (+ 1 (stack-depth bco))))
      (stack-depth-set! bco new-depth)
      new-depth))

  (define (emit-primitive bco prim args)
    #;(for-each
    (lambda (x) (emit bco 'load x))
    args)
    (emit bco prim))

  ;; Emit bindings.
  ;; Args: `bco` = bytecode object, `variables` = variables being bound
  ;; `env` = environment
  (define (emit-bindings bco variables expressions env compile-form)
    (assert (proper-list? variables))
    (assert (proper-list? expressions))
    (assert (= (length variables) (length expressions)))
    (let ((depth (stack-depth bco)))
      (for-each
       (lambda (var expr)
         (compile-form expr)
         (if (> (stack-depth bco) depth)
             (emit bco 'adjust-stack (- (stack-depth bco) depth)))
         (set! depth (+ 1 depth))
         (stack-depth-set! bco depth)
         (hash-table-set! env var depth))
       variables expressions)
      depth))

  (define (emit-variable-reference bco stack-position)
    (assert (bco? bco))
    (let ((new-stack-depth (+ 1 (stack-depth bco))))
      (stack-depth-set! bco new-stack-depth)
      (cond
       ((symbol? stack-position)
        (emit bco 'global-load stack-position))
       ((fixnum? stack-position)
        (emit bco 'load stack-position))
       (else (assert #f)))))

  (define (emit-constant bco object)
    (case object
      ((#f) (emit bco 'load-f))
      ((#t) (emit bco 'load-t))
      ((0)  (emit bco 'load-0))
      ((1)  (emit bco 'load-1))
      (else ; Memoize the objects using the bytecode object's memo table
       (let ((index (hash-table-ref (memo bco) object (lambda () #f))))
         (emit bco 'load-constant-index
               (if index index (add-to-constant-vector bco object)))))))

  (define (emit-set! bco stack-position other-stack-position)
    (cond
     ((symbol? stack-position)
      (emit bco 'global-load stack-position other-stack-position))
     ((fixnum? stack-position)
      (emit bco 'load stack-position other-stack-position))
     (else (error 'assert "invalid stack position"))))

  (define (emit-lambda-definition bco variadic? fixed-args body)
    (let ((stack-position (stack-depth bco))
          (label-start (incr-counter bco))
          (label-end (incr-counter bco)))
      (emit bco 'closure label-start label-end variadic? fixed-args)
      (emit bco 'label label-start)
      (body)
      (emit bco 'label label-end)))

  (define (emit-apply bco function args)
    (emit bco 'apply function args))

  (define (incr-counter bco)
    (let ((old-val (counter bco)))
      (counter-set! bco (+ 1 old-val))
      old-val))
  ;; Emit a jump.
  (define (emit-jump bco condition yes no)
    (let ((stack-position (stack-depth bco))
          (label-true (incr-counter bco))
          (label-false (incr-counter bco)))
      (condition)
      (emit bco 'branch label-true (+ 1 stack-position))
      (no)
      (emit bco 'jump label-false)
      (emit bco 'label label-true)
      (yes)
      (emit bco 'label label-false))))
;;; Local Variables:
;;; mode: scheme
;;; End:
