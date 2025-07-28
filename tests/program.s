    .globl main
main:
    pushq %rbp
    movq %rsp, %rbp
    subq $16, %rsp
    movl $1, %r11d
    cmpl $2, %r11d
    movl $0, -4(%rbp)
    setg -4(%rbp)
    cmpl $0, -4(%rbp)
    je .Llabel1
    movl $3, %r11d
    cmpl $4, %r11d
    movl $0, -8(%rbp)
    setle -8(%rbp)
    cmpl $0, -8(%rbp)
    je .Llabel1
    movl $1, -12(%rbp)
    jmp .Llabel2
    .Llabel1:
    movl $0, -12(%rbp)
    .Llabel2:
    movl -12(%rbp), %eax
    movq %rbp, %rsp
    popq %rbp
    ret

.section .note.GNU-stack,"",@progbits
