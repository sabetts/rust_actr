(clear-all)

(define-model test-them-all
  (sgp :dcnn nil :use-tree nil :style-warnings nil)

  (chunk-type test slot1 slot2 slot3)
  
  (define-chunks (dummy1 isa chunk)
      (g1 isa test)
    (merged isa chunk)
    (slot1 isa chunk)
    (slot2 isa chunk)
    (slot3 isa chunk)
    (not-a-slot isa chunk))
  
  (merge-chunks dummy1 merged)
  
  (p slot1=chunk
     =goal>
     isa test
     slot1 dummy1
     ==>)
  
  (p slot1=implicit-chunk
     =goal>
     isa test
     slot1 dummy2
     ==>)
  
  (p slot1=string
     =goal>
     isa test
     slot1 "dummy1"
     ==>)
  
  (p slot1=number
     =goal>
     isa test
     slot1 5
     ==>)
  
  (p slot1=t
     =goal>
     isa test
     slot1 t
     ==>)
  
  (p slot1=nil
     =goal>
     isa test
     slot1 nil
     ==>)
  
  
  (p slot1-chunk
     =goal>
     isa test
     - slot1 dummy1
     ==>)
  
  (p slot1-implicit-chunk
     =goal>
     isa test
     - slot1 dummy2
     ==>)
  
  (p slot1-string
     =goal>
     isa test
     - slot1 "dummy1"
     ==>)
  
  (p slot1-number
     =goal>
     isa test
     - slot1 5
     ==>)
  
  (p slot1-t
     =goal>
     isa test
     - slot1 t
     ==>)
  
  (p slot1-nil
     =goal>
     isa test
     - slot1 nil
     ==>)
  
  (p slot1<chunk
     =goal>
     isa test
     < slot1 dummy1
     ==>)
  
  (p slot1<implicit-chunk
     =goal>
     isa test
     < slot1 dummy2
     ==>)
  
  (p slot1<string
     =goal>
     isa test
     < slot1 "dummy1"
     ==>)
  
  (p slot1<number
     =goal>
     isa test
     < slot1 5
     ==>)
  
  (p slot1<t
     =goal>
     isa test
     < slot1 t
     ==>)
  
  (p slot1<nil
     =goal>
     isa test
     < slot1 nil
     ==>)
  
  (p slot1<=chunk
     =goal>
     isa test
     <= slot1 dummy1
     ==>)
  
  (p slot1<=implicit-chunk
     =goal>
     isa test
     <= slot1 dummy2
     ==>)
  
  (p slot1<=string
     =goal>
     isa test
     <= slot1 "dummy1"
     ==>)
  
  (p slot1<=number
     =goal>
     isa test
     <= slot1 5
     ==>)
  
  (p slot1<=t
     =goal>
     isa test
     <= slot1 t
     ==>)
  
  (p slot1<=nil
     =goal>
     isa test
     <= slot1 nil
     ==>)
  
  (p slot1>chunk
     =goal>
     isa test
     > slot1 dummy1
     ==>)
  
  (p slot1>implicit-chunk
     =goal>
     isa test
     > slot1 dummy2
     ==>)
  
  (p slot1>string
     =goal>
     isa test
     > slot1 "dummy1"
     ==>)
  
  (p slot1>number
     =goal>
     isa test
     > slot1 5
     ==>)
  
  (p slot1>t
     =goal>
     isa test
     > slot1 t
     ==>)
  
  (p slot1>nil
     =goal>
     isa test
     > slot1 nil
     ==>)

  
  (p slot1=>chunk
     =goal>
     isa test
     >= slot1 dummy1
     ==>)
  
  (p slot1>=implicit-chunk
     =goal>
     isa test
     >= slot1 dummy2
     ==>)
  
  (p slot1>=string
     =goal>
     isa test
     >= slot1 "dummy1"
     ==>)
  
  (p slot1>=number
     =goal>
     isa test
     >= slot1 5
     ==>)
  
  (p slot1>=t
     =goal>
     isa test
     >= slot1 t
     ==>)
  
  (p slot1>=nil
     =goal>
     isa test
     >= slot1 nil
     ==>)
  
  ;;;;;;;;;;
  
  (p p*-slot1=chunk
     =goal>
     isa test
     slot1 dummy1
     ==>)
  
  (p p*-slot1=implicit-chunk
     =goal>
     isa test
     slot1 dummy3
     ==>)
  
  (p p*-slot1=string
     =goal>
     isa test
     slot1 "dummy1"
     ==>)
  
  (p p*-slot1=number
     =goal>
     isa test
     slot1 5
     ==>)
  
  (p p*-slot1=t
     =goal>
     isa test
     slot1 t
     ==>)
  
  (p p*-slot1=nil
     =goal>
     isa test
     slot1 nil
     ==>)
  
  
  (p p*-slot1-chunk
     =goal>
     isa test
     - slot1 dummy1
     ==>)
  
  (p p*-slot1-implicit-chunk
     =goal>
     isa test
     - slot1 dummy3
     ==>)
  
  (p p*-slot1-string
     =goal>
     isa test
     - slot1 "dummy1"
     ==>)
  
  (p p*-slot1-number
     =goal>
     isa test
     - slot1 5
     ==>)
  
  (p p*-slot1-t
     =goal>
     isa test
     - slot1 t
     ==>)
  
  (p p*-slot1-nil
     =goal>
     isa test
     - slot1 nil
     ==>)
  
  (p p*-slot1<chunk
     =goal>
     isa test
     < slot1 dummy1
     ==>)
  
  (p p*-slot1<implicit-chunk
     =goal>
     isa test
     < slot1 dummy3
     ==>)
  
  (p p*-slot1<string
     =goal>
     isa test
     < slot1 "dummy1"
     ==>)
  
  (p p*-slot1<number
     =goal>
     isa test
     < slot1 5
     ==>)
  
  (p p*-slot1<t
     =goal>
     isa test
     < slot1 t
     ==>)
  
  (p p*-slot1<nil
     =goal>
     isa test
     < slot1 nil
     ==>)
  
  (p p*-slot1<=chunk
     =goal>
     isa test
     <= slot1 dummy1
     ==>)
  
  (p p*-slot1<=implicit-chunk
     =goal>
     isa test
     <= slot1 dummy3
     ==>)
  
  (p p*-slot1<=string
     =goal>
     isa test
     <= slot1 "dummy1"
     ==>)
  
  (p p*-slot1<=number
     =goal>
     isa test
     <= slot1 5
     ==>)
  
  (p p*-slot1<=t
     =goal>
     isa test
     <= slot1 t
     ==>)
  
  (p p*-slot1<=nil
     =goal>
     isa test
     <= slot1 nil
     ==>)
  
  (p p*-slot1>chunk
     =goal>
     isa test
     > slot1 dummy1
     ==>)
  
  (p p*-slot1>implicit-chunk
     =goal>
     isa test
     > slot1 dummy3
     ==>)
  
  (p p*-slot1>string
     =goal>
     isa test
     > slot1 "dummy1"
     ==>)
  
  (p p*-slot1>number
     =goal>
     isa test
     > slot1 5
     ==>)
  
  (p p*-slot1>t
     =goal>
     isa test
     > slot1 t
     ==>)
  
  (p p*-slot1>nil
     =goal>
     isa test
     > slot1 nil
     ==>)

  
  (p p*-slot1=>chunk
     =goal>
     isa test
     >= slot1 dummy1
     ==>)
  
  (p p*-slot1>=implicit-chunk
     =goal>
     isa test
     >= slot1 dummy3
     ==>)
  
  (p p*-slot1>=string
     =goal>
     isa test
     >= slot1 "dummy1"
     ==>)
  
  (p p*-slot1>=number
     =goal>
     isa test
     >= slot1 5
     ==>)
  
  (p p*-slot1>=t
     =goal>
     isa test
     >= slot1 t
     ==>)
  
  (p p*-slot1>=nil
     =goal>
     isa test
     >= slot1 nil
     ==>)
  
  
  ;;;;;   
  
  
  (p p*-implicit-slot=chunk
      =goal>
      isa test
      slot2 =slot
      =slot dummy1
      ==>)
  
  (p p*-implicit-slot=implicit-chunk
     =goal>
      isa test
      slot2 =slot
      =slot dummy4
     ==>)
  
  (p p*-implicit-slot=string
     =goal>
       isa test
      slot2 =slot
     =slot "dummy1"
     ==>)
  
  (p p*-implicit-slot=number
     =goal>
       isa test
      slot2 =slot
     =slot 5
     ==>)
  
  (p p*-implicit-slot=t
     =goal>
       isa test
      slot2 =slot
     =slot t
     ==>)
  
  (p p*-implicit-slot=nil
     =goal>
       isa test
      slot2 =slot
     =slot nil
     ==>)
  
  
  (p p*-implicit-slot-chunk
     =goal>
       isa test
      slot2 =slot
     - =slot dummy1
     ==>)
  
  (p p*-implicit-slot-implicit-chunk
     =goal>
       isa test
      slot2 =slot
     - =slot dummy4
     ==>)
  
  (p p*-implicit-slot-string
     =goal>
       isa test
      slot2 =slot
     - =slot "dummy1"
     ==>)
  
  (p p*-implicit-slot-number
     =goal>
       isa test
      slot2 =slot
     - =slot 5
     ==>)
  
  (p p*-implicit-slot-t
     =goal>
       isa test
      slot2 =slot
     - =slot t
     ==>)
  
  (p p*-implicit-slot-nil
     =goal>
       isa test
      slot2 =slot
     - =slot nil
     ==>)
  
  (p p*-implicit-slot<chunk
     =goal>
       isa test
      slot2 =slot
     < =slot dummy1
     ==>)
  
  (p p*-implicit-slot<implicit-chunk
     =goal>
       isa test
      slot2 =slot
     < =slot dummy4
     ==>)
  
  (p p*-implicit-slot<string
     =goal>
       isa test
      slot2 =slot
     < =slot "dummy1"
     ==>)
  
  (p p*-implicit-slot<number
     =goal>
       isa test
      slot2 =slot
     < =slot 5
     ==>)
  
  (p p*-implicit-slot<t
     =goal>
       isa test
      slot2 =slot
     < =slot t
     ==>)
  
  (p p*-implicit-slot<nil
     =goal>
       isa test
      slot2 =slot
     < =slot nil
     ==>)
  
  (p p*-implicit-slot<=chunk
     =goal>
       isa test
      slot2 =slot
     <= =slot dummy1
     ==>)
  
  (p p*-implicit-slot<=implicit-chunk
     =goal>
       isa test
      slot2 =slot
     <= =slot dummy4
     ==>)
  
  (p p*-implicit-slot<=string
     =goal>
       isa test
      slot2 =slot
     <= =slot "dummy1"
     ==>)
  
  (p p*-implicit-slot<=number
     =goal>
       isa test
      slot2 =slot
     <= =slot 5
     ==>)
  
  (p p*-implicit-slot<=t
     =goal>
       isa test
      slot2 =slot
     <= =slot t
     ==>)
  
  (p p*-implicit-slot<=nil
     =goal>
       isa test
      slot2 =slot
     <= =slot nil
     ==>)
  
  (p p*-implicit-slot>chunk
     =goal>
       isa test
      slot2 =slot
     > =slot dummy1
     ==>)
  
  (p p*-implicit-slot>implicit-chunk
     =goal>
       isa test
      slot2 =slot
     > =slot dummy4
     ==>)
  
  (p p*-implicit-slot>string
     =goal>
       isa test
      slot2 =slot
     > =slot "dummy1"
     ==>)
  
  (p p*-implicit-slot>number
     =goal>
       isa test
      slot2 =slot
     > =slot 5
     ==>)
  
  (p p*-implicit-slot>t
     =goal>
       isa test
      slot2 =slot
     > =slot t
     ==>)
  
  (p p*-implicit-slot>nil
     =goal>
       isa test
      slot2 =slot
     > =slot nil
     ==>)

  
  (p p*-implicit-slot=>chunk
     =goal>
       isa test
      slot2 =slot
     >= =slot dummy1
     ==>)
  
  (p p*-implicit-slot>=implicit-chunk
     =goal>
       isa test
      slot2 =slot
     >= =slot dummy4
     ==>)
  
  (p p*-implicit-slot>=string
     =goal>
       isa test
      slot2 =slot
     >= =slot "dummy1"
     ==>)
  
  (p p*-implicit-slot>=number
     =goal>
       isa test
      slot2 =slot
     >= =slot 5
     ==>)
  
  (p p*-implicit-slot>=t
     =goal>
       isa test
      slot2 =slot
     >= =slot t
     ==>)
  
  (p p*-implicit-slot>=nil
     =goal>
       isa test
      slot2 =slot
     >= =slot nil
     ==>)
)
