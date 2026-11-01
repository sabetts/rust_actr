"""Python port of ACT-R:tutorial;unit1;addition.lisp."""

import rust_actr as actr

m = actr.Model('addition')

m.sgp(':esc', True)
m.sgp(':lf', 0.05)

m.chunk_type('number', 'number', 'next')
m.chunk_type('add', 'arg1', 'arg2', 'sum', 'count')

m.add_chunk('zero', 'number', number='zero', next='one')
m.add_chunk('one', 'number', number='one', next='two')
m.add_chunk('two', 'number', number='two', next='three')
m.add_chunk('three', 'number', number='three', next='four')
m.add_chunk('four', 'number', number='four', next='five')
m.add_chunk('five', 'number', number='five', next='six')
m.add_chunk('six', 'number', number='six', next='seven')
m.add_chunk('seven', 'number', number='seven', next='eight')
m.add_chunk('eight', 'number', number='eight', next='nine')
m.add_chunk('nine', 'number', number='nine', next='ten')
m.add_chunk('ten', 'number', number='ten')

m.goal_focus(isa='add', arg1='five', arg2='two')

m.add_production(
    '''
    (p initialize-addition
       =goal>
          isa         add
          arg1        =num1
          arg2        =num2
          sum         nil
      ==>
       =goal>
          isa         add
          sum         =num1
          count       zero
       +retrieval>
          isa         number
          number      =num1
    )
    '''
)

m.add_production(
    '''
    (p terminate-addition
       =goal>
          isa         add
          count       =num
          arg2        =num
          sum         =answer
       =retrieval>
          isa         number
          number      =answer
      ==>
       =goal>
          isa         add
          count       nil
       !output!       (=answer)
    )
    '''
)

m.add_production(
    '''
    (p increment-count
       =goal>
          isa         add
          sum         =sum
          count       =count
       =retrieval>
          isa         number
          number      =count
          next        =newcount
      ==>
       =goal>
          isa         add
          count       =newcount
       +retrieval>
          isa         number
          number      =sum
    )
    '''
)

m.add_production(
    '''
    (p increment-sum
       =goal>
          isa         add
          sum         =sum
          count       =count
        - arg2        =count
       =retrieval>
          isa         number
          number      =sum
          next        =newsum
      ==>
       =goal>
          isa         add
          sum         =newsum
       +retrieval>
          isa         number
          number      =count
    )
    '''
)


if __name__ == "__main__":
    print(m.run(10.0), end="")
