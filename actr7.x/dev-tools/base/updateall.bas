REM  *****  BASIC  *****

Sub Main

End Sub


sub updateall
rem ----------------------------------------------------------------------
rem define variables
dim document   as object
dim dispatcher as object
rem ----------------------------------------------------------------------
rem get access to the document
document   = ThisComponent.CurrentController.Frame
dispatcher = createUnoService("com.sun.star.frame.DispatchHelper")

rem ----------------------------------------------------------------------
dispatcher.executeDispatch(document, ".uno:UpdateAll", "", 0, Array())
dispatcher.executeDispatch(document, ".uno:Save", "", 0, Array())
ThisComponent.close(True)

end sub


sub saveaspdf
rem ----------------------------------------------------------------------
rem define variables
dim document   as object
dim dispatcher as object
rem ----------------------------------------------------------------------
rem get access to the document
document   = ThisComponent.CurrentController.Frame
dispatcher = createUnoService("com.sun.star.frame.DispatchHelper")

rem ----------------------------------------------------------------------
dim args1(1) as new com.sun.star.beans.PropertyValue
args1(0).Name = "URL"
args1(0).Value = "file:///C:/Users/db30/add.pdf"
args1(1).Name = "FilterName"
args1(1).Value = "writer_pdf_Export"

dispatcher.executeDispatch(document, ".uno:ExportDirectToPDF", "", 0, args1())


end sub
