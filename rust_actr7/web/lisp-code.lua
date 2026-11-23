function CodeBlock(block)
  local text = block.text
  if text:match("^%s*%(") or text:match("^%s*[?>]%s*%(") then
    if not block.classes:includes("lisp") then
      block.classes:insert("lisp")
    end
  end
  return block
end
