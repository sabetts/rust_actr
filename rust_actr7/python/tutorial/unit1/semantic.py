"""Python port of ACT-R:tutorial;unit1;semantic.lisp."""

import rust_actr as actr

m = actr.Model('semantic')

m.sgp('esc', True)
m.sgp('lf', 0.05)

m.chunk_type('property', 'object', 'attribute', 'value')
m.chunk_type('is-member', 'object', 'category', 'judgment')

m.add_dm('shark')
m.add_dm('dangerous')
m.add_dm('locomotion')
m.add_dm('swimming')
m.add_dm('fish')
m.add_dm('salmon')
m.add_dm('edible')
m.add_dm('breathe')
m.add_dm('gills')
m.add_dm('animal')
m.add_dm('moves')
m.add_dm('skin')
m.add_dm('canary')
m.add_dm('color')
m.add_dm('sings')
m.add_dm('bird')
m.add_dm('ostrich')
m.add_dm('flies')
m.add_dm('height')
m.add_dm('tall')
m.add_dm('wings')
m.add_dm('flying')
m.add_dm('true')
m.add_dm('false')
m.add_chunk('p1', 'property', object='shark', attribute='dangerous', value='true')
m.add_chunk('p2', 'property', object='shark', attribute='locomotion', value='swimming')
m.add_chunk('p3', 'property', object='shark', attribute='category', value='fish')
m.add_chunk('p4', 'property', object='salmon', attribute='edible', value='true')
m.add_chunk('p5', 'property', object='salmon', attribute='locomotion', value='swimming')
m.add_chunk('p6', 'property', object='salmon', attribute='category', value='fish')
m.add_chunk('p7', 'property', object='fish', attribute='breathe', value='gills')
m.add_chunk('p8', 'property', object='fish', attribute='locomotion', value='swimming')
m.add_chunk('p9', 'property', object='fish', attribute='category', value='animal')
m.add_chunk('p10', 'property', object='animal', attribute='moves', value='true')
m.add_chunk('p11', 'property', object='animal', attribute='skin', value='true')
m.add_chunk('p12', 'property', object='canary', attribute='color', value='yellow')
m.add_chunk('p13', 'property', object='canary', attribute='sings', value='true')
m.add_chunk('p14', 'property', object='canary', attribute='category', value='bird')
m.add_chunk('p15', 'property', object='ostrich', attribute='flies', value='false')
m.add_chunk('p16', 'property', object='ostrich', attribute='height', value='tall')
m.add_chunk('p17', 'property', object='ostrich', attribute='category', value='bird')
m.add_chunk('p18', 'property', object='bird', attribute='wings', value='true')
m.add_chunk('p19', 'property', object='bird', attribute='locomotion', value='flying')
m.add_chunk('p20', 'property', object='bird', attribute='category', value='animal')

m.goal_focus(isa='is-member', object='canary', category='bird')

m.add_production(
    '''
    (p initial-retrieve
       =goal>
          isa         is-member
          object      =obj
          category    =cat
          judgment    nil
    ==>
       =goal>
          judgment    pending
       +retrieval>
          isa         property
          object      =obj
          attribute   category
    )
    '''
)

m.add_production(
    '''
    (p direct-verify
       =goal>
          isa         is-member
          object      =obj
          category    =cat
          judgment    pending
       =retrieval>
          isa         property
          object      =obj
          attribute   category
          value       =cat
    ==>
       =goal>
          judgment    yes
    )
    '''
)

m.add_production(
    '''
    (p chain-category
       =goal>
          isa         is-member
          object      =obj1
          category    =cat
          judgment    pending
       =retrieval>
          isa         property
          object      =obj1
          attribute   category
          value       =obj2
        - value       =cat
    ==>
       =goal>
          object      =obj2
       +retrieval>
          isa         property
          object      =obj2
          attribute   category
    )
    '''
)

m.add_production(
    '''
    (p fail
       =goal>
          isa         is-member
          object      =obj1
          category    =cat
          judgment    pending

       ?retrieval>
          buffer      failure
    ==>
       =goal>
          judgment    no
    )
    '''
)


if __name__ == "__main__":
    print(m.run(10.0), end="")
