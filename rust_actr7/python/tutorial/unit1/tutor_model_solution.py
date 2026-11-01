"""Python port of ACT-R:tutorial;unit1;tutor-model-solution.lisp."""

import rust_actr as actr

m = actr.Model('tutor-model')

m.sgp('esc', True)
m.sgp('lf', 0.05)
m.sgp('trace_detail', 'medium')

m.chunk_type('addition-fact', 'addend1', 'addend2', 'sum')
m.chunk_type('add-pair', 'one1', 'ten1', 'one2', 'ten2', 'ten-ans', 'one-ans', 'carry')

m.add_chunk('fact17', 'addition-fact', addend1=1, addend2=7, sum=8)
m.add_chunk('fact34', 'addition-fact', addend1=3, addend2=4, sum=7)
m.add_chunk('fact67', 'addition-fact', addend1=6, addend2=7, sum=13)
m.add_chunk('fact103', 'addition-fact', addend1=10, addend2=3, sum=13)

m.add_production(
    '''
    (p start-pair
      =goal>
        isa add-pair
        one1 =num1
        one2 =num2
        one-ans nil
    ==>
      =goal>
        one-ans busy
      +retrieval>
        isa addition-fact
        addend1 =num1
        addend2 =num2)
    '''
)

m.add_production(
    '''
    (p add-ones
      =goal>
        isa add-pair
        one-ans busy
        one1 =num1
        one2 =num2
      =retrieval>
        isa addition-fact
        addend1 =num1
        addend2 =num2
        sum =sum
    ==>
      =goal>
        one-ans =sum
        carry busy
      +retrieval>
        isa addition-fact
        addend1 10
        sum =sum)
    '''
)

m.add_production(
    '''
    (p process-carry
      =goal>
        isa add-pair
        ten1 =num1
        ten2 =num2
        carry busy
        one-ans =ones
      =retrieval>
        isa addition-fact
        addend1 10
        sum =ones
        addend2 =remainder
    ==>
      =goal>
        carry 1
        ten-ans busy
        one-ans =remainder
      +retrieval>
        isa addition-fact
        addend1 =num1
        addend2 =num2)
    '''
)

m.add_production(
    '''
    (p no-carry
      =goal>
        isa add-pair
        ten1 =num1
        ten2 =num2
        carry busy
      ?retrieval>
        buffer failure
    ==>
      =goal>
        carry nil
        ten-ans busy
      +retrieval>
        isa addition-fact
        addend1 =num1
        addend2 =num2)
    '''
)

m.add_production(
    '''
    (p add-tens-done
      =goal>
        isa add-pair
        ten-ans busy
        carry nil
      =retrieval>
        isa addition-fact
        sum =sum
    ==>
      =goal>
        ten-ans =sum)
    '''
)

m.add_production(
    '''
    (p add-tens-carry
      =goal>
        isa add-pair
        carry 1
        ten-ans busy
      =retrieval>
        isa addition-fact
        sum =sum
    ==>
      =goal>
        carry nil
      +retrieval>
        isa addition-fact
        addend1 1
        addend2 =sum)
    '''
)

m.goal_focus(isa='add-pair', ten1=3, one1=6, ten2=4, one2=7)


if __name__ == "__main__":
    print(m.run(10.0), end="")
