local status_ok, config = pcall(require, 'kommentary.config')
if not status_ok then
  return
end

config.use_extended_mappings()
