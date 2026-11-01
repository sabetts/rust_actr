"""Python port of ACT-R:tutorial;unit1;count.lisp."""

import rust_actr as actr

m = actr.Model('count')

m.sgp('esc', True)
m.sgp('lf', 0.05)
m.sgp('trace_detail', 'high')

m.chunk_type('number', 'number', 'next')
m.chunk_type('count-from', 'start', 'end', 'count')

m.add_chunk('one', 'number', number='one', next='two')
m.add_chunk('two', 'number', number='two', next='three')
m.add_chunk('three', 'number', number='three', next='four')
m.add_chunk('four', 'number', number='four', next='five')
m.add_chunk('five', 'number', number='five')

m.goal_focus(isa='count-from', start='two', end='four')

m.add_production(
    '''
    (p start
       =goal>
          isa         count-from
          start       =num1
          count       nil
     ==>
       =goal>
          isa         count-from
          count       =num1
       +retrieval>
          isa         number
          number      =num1
       )
    '''
)

m.add_production(
    '''
    (p increment
       =goal>
          isa         count-from
          count       =num1
        - end         =num1
       =retrieval>
          isa         number
          number      =num1
          next        =num2
     ==>
       =goal>
          isa         count-from
          count       =num2
       +retrieval>
          isa         number
          number      =num2
       !output!       (=num1)
       )
    '''
)

m.add_production(
    '''
    (p stop
       =goal>
          isa         count-from
          count       =num
          end         =num
       =retrieval>
          isa         number
          number      =num
     ==>
       -goal>
       !output!       (=num)
       )
    '''
)


if __name__ == "__main__":
    print(m.run(10.0), end="")
