"""Python port of ACT-R:tutorial;unit8;semantic-dynamic-model.lisp."""

import rust_actr as actr

m = actr.Model('semantic')

m.sgp(':esc', True)
m.sgp(':lf', 0.05)

m.chunk_type('property', 'object', 'category', 'dangerous', 'locomotion', 'edible', 'breathe', 'moves', 'skin', 'color', 'sings', 'flies', 'height', 'wings')
m.chunk_type('test-attribute', 'object', 'original', 'attribute', 'value', 'state', 'answer')

m.define_chunks('shark')
m.define_chunks('salmon')
m.define_chunks('fish')
m.define_chunks('animal')
m.define_chunks('canary')
m.define_chunks('ostrich')
m.define_chunks('bird')
m.define_chunks('dangerous')
m.define_chunks('locomotion')
m.define_chunks('category')
m.define_chunks('edible')
m.define_chunks('breathe')
m.define_chunks('moves')
m.define_chunks('skin')
m.define_chunks('color')
m.define_chunks('sings')
m.define_chunks('flies')
m.define_chunks('height')
m.define_chunks('wings')
m.define_chunks('gills')
m.define_chunks('swimming')
m.define_chunks('tall')
m.define_chunks('flying')
m.define_chunks('true')
m.define_chunks('false')
m.define_chunks('yes')
m.define_chunks('no')
m.define_chunks('dont-know')
m.define_chunks('attr-check')
m.define_chunks('record')
m.define_chunks('create')
m.define_chunks('done')
m.define_chunks('check-category')
m.add_chunk('p1', 'property', object='shark', dangerous='true')
m.add_chunk('p2', 'property', object='shark', locomotion='swimming')
m.add_chunk('p3', 'property', object='shark', category='fish')
m.add_chunk('p4', 'property', object='salmon', edible='true')
m.add_chunk('p5', 'property', object='salmon', locomotion='swimming')
m.add_chunk('p6', 'property', object='salmon', category='fish')
m.add_chunk('p7', 'property', object='fish', breathe='gills')
m.add_chunk('p8', 'property', object='fish', locomotion='swimming')
m.add_chunk('p9', 'property', object='fish', category='animal')
m.add_chunk('p10', 'property', object='animal', moves='true')
m.add_chunk('p11', 'property', object='animal', skin='true')
m.add_chunk('p12', 'property', object='canary', color='yellow')
m.add_chunk('p13', 'property', object='canary', sings='true')
m.add_chunk('p14', 'property', object='canary', category='bird')
m.add_chunk('p15', 'property', object='ostrich', flies='false')
m.add_chunk('p16', 'property', object='ostrich', height='tall')
m.add_chunk('p17', 'property', object='ostrich', category='bird')
m.add_chunk('p18', 'property', object='bird', wings='true')
m.add_chunk('p19', 'property', object='bird', locomotion='flying')
m.add_chunk('p20', 'property', object='bird', category='animal')
m.add_chunk('p21', 'property', object='bird', flies='true')
m.define_chunks(actr.Chunk('g1', 'test-attribute', {'object': 'canary', 'attribute': 'category', 'value': 'bird'}))
m.define_chunks(actr.Chunk('g2', 'test-attribute', {'object': 'canary', 'attribute': 'wings', 'value': 'true'}))
m.define_chunks(actr.Chunk('g3', 'test-attribute', {'object': 'canary', 'attribute': 'dangerous', 'value': 'true'}))

m.declare_buffer_usage('goal', 'test-attribute', ':all')

m.add_production(
    '''
    (p retrieve-attribute
       =goal>
         isa         test-attribute
         object      =obj
         attribute   =attr
         value       =val
         state       nil
      ==>
       =goal>
         state       attr-check
       +retrieval>
         isa         property
         object      =obj
         - =attr     nil)
    '''
)

m.add_production(
    '''
    (p direct-verify
       =goal>
         isa         test-attribute
         original    nil
         object      =obj
         attribute   =attr
         value       =val
         state       attr-check
       =retrieval>
         isa         property
         object      =obj
         =attr       =val
      ==>
       =goal>
         answer      yes
         state       done)
    '''
)

m.add_production(
    '''
    (p indirect-verify
       =goal>
         isa         test-attribute
         original    =ori
         object      =obj
         attribute   =attr
         value       =val
         state       attr-check
       =retrieval>
         isa         property
         object      =obj
         =attr       =val
    ==>
       =goal>
         object      =ori
         answer      yes
         state       record)
    '''
)

m.add_production(
    '''
    (p record-result
       =goal>
         isa         test-attribute
         object      =obj
         attribute   =attr
         value       =val
         state       record
       ?imaginal>
         state       free
      ==>
       +imaginal>
         object      =obj
         =attr       =val
       =goal>
         answer      yes
         state       create)
    '''
)

m.add_production(
    '''
    (p create
       =goal>
         isa         test-attribute
         state       create
       ?imaginal>
         state       free
         buffer      full
      ==>
       -imaginal>
       =goal>
         state       done)
    '''
)

m.add_production(
    '''
    (p disprove
       =goal>
         isa         test-attribute
         object      =obj
         attribute   =attr
         value       =val
         state       attr-check
       =retrieval>
         isa         property
         object      =obj
         - =attr     =val
      ==>
       =goal>
         answer      no
         state       done)
    '''
)

m.add_production(
    '''
    (p missing-attr
       =goal>
         isa         test-attribute
         object      =obj
         state       attr-check
       ?retrieval>
         buffer      failure
      ==>
       =goal>
         state       check-category
       +retrieval>
         isa         property
         object      =obj
         - category  nil
    )
    '''
)

m.add_production(
    '''
    (p chain-first-category
       =goal>
         isa         test-attribute
         original    nil
         object      =obj
         state       check-category
       =retrieval>
         isa         property
         object      =obj
         category    =obj2
      ==>
       =goal>
         original    =obj
         object      =obj2
         state       nil)
    '''
)

m.add_production(
    '''
    (p chain-subsequent-category
       =goal>
         isa         test-attribute
         - original  nil
         object      =obj
         state       check-category
       =retrieval>
         isa         property
         object      =obj
         category    =obj2
      ==>
       =goal>
         object      =obj2
         state       nil)
    '''
)

m.add_production(
    '''
    (p failed-to-find-category
      =goal>
        isa         test-attribute
        state       check-category
       ?retrieval>
        buffer      failure
     ==>
      =goal>
        answer      dont-know
        state       done)
    '''
)

m.add_production(
    '''
    (p respond
       =goal>
         isa         test-attribute
         state       done
         object      =obj
         attribute   =attr
         value       =val
         answer      =ans
      ==>
       !output! (=obj =attr =val answer is =ans)
       -goal>)
    '''
)


if __name__ == "__main__":
    print(m.run(10.0), end="")
