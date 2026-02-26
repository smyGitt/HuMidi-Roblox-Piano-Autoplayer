import os
import sys
import json
from pathlib import Path

class ConfigManager:
    def __init__(self):
        # Resolve path regardless of whether the script is run natively or compiled to .exe via PyInstaller
        if getattr(sys, 'frozen', False):
            self.root_dir = os.path.dirname(sys.executable)
        else:
            self.root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
            
        self.save_dir = os.path.join(self.root_dir, 'saves')
        os.makedirs(self.save_dir, exist_ok=True)
        
        self.config_dir = Path.home() / ".humidi"
        self.config_path = self.config_dir / "config.json"
        self.config_dir.mkdir(exist_ok=True)

    def load(self) -> dict:
        """Reads configuration data from Windows local appdata."""
        if not self.config_path.exists(): 
            return {}
        try:
            with open(self.config_path, 'r') as f:
                config = json.load(f)
                if config.get('save_dir') and os.path.exists(config.get('save_dir')):
                    self.save_dir = config.get('save_dir')
                return config
        except Exception:
            return {}

    def save(self, config_data: dict):
        """Persists the current application state to JSON."""
        config_data['save_dir'] = self.save_dir
        try:
            with open(self.config_path, 'w') as f:
                json.dump(config_data, f, indent=4)
        except Exception as e:
            print(f"Error saving config: {e}")

    def set_save_dir(self, new_dir: str):
        self.save_dir = new_dir