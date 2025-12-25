require "process"

def get_gpu_driver
  output = `lspci -nnk | grep -A3 'VGA compatible controller'`
  lines = output.split("\n")
  driver = nil
  lines.each do |line|
    if line.starts_with?("\tKernel driver in use:")
      driver = line.split(":")[1].strip
      break
    end
  end
  driver
end

driver = get_gpu_driver
app_path = "/usr/lib/HackerOS/Hacker-Mode/bin/Hacker-Mode.AppImage"

# Assume old drivers: nouveau, radeon, i915
if driver && ["nouveau", "radeon", "i915"].includes?(driver.downcase)
  puts "Detected old GPU with driver: #{driver}. Running with gameframe."
  Process.run("gameframe", args: [app_path])
else
  puts "Detected new GPU with driver: #{driver || 'unknown'}. Running with cage."
  Process.run("cage", args: [app_path])
end
