import React from 'react';
import { Power, RefreshCw, Moon, MonitorX, X } from 'lucide-react';
import { Translation } from '../types';
import { systemService } from '../services/systemService';

interface PowerMenuProps {
    t: Translation;
    onClose: () => void;
}

export const PowerMenu: React.FC<PowerMenuProps> = ({ t, onClose }) => {
    const handleAction = (action: string) => {
        systemService.systemAction(action);
        onClose();
    };

    return (
        <div className="fixed inset-0 z-[60] bg-black/80 backdrop-blur-md flex items-center justify-center animate-fade-in">
        <div className="w-full max-w-lg bg-[#1e2329] border border-gray-700 rounded-xl shadow-2xl overflow-hidden p-8">
        <div className="flex justify-between items-center mb-8">
        <h2 className="text-2xl font-bold text-white">System Menu</h2>
        <button onClick={onClose} className="p-2 hover:bg-white/10 rounded-full">
        <X />
        </button>
        </div>

        <div className="grid grid-cols-1 gap-3">
        <button onClick={() => handleAction('sleep')} className="flex items-center gap-4 p-4 rounded bg-gray-800 hover:bg-steam-blue hover:text-white transition-colors group">
        <Moon className="text-blue-400 group-hover:text-white" />
        <span className="font-bold text-lg">{t.system.sleep}</span>
        </button>
        <button onClick={() => handleAction('restart')} className="flex items-center gap-4 p-4 rounded bg-gray-800 hover:bg-steam-blue hover:text-white transition-colors group">
        <RefreshCw className="text-yellow-400 group-hover:text-white" />
        <span className="font-bold text-lg">{t.system.restart}</span>
        </button>
        <button onClick={() => handleAction('shutdown')} className="flex items-center gap-4 p-4 rounded bg-gray-800 hover:bg-steam-blue hover:text-white transition-colors group">
        <Power className="text-red-400 group-hover:text-white" />
        <span className="font-bold text-lg">{t.system.shutdown}</span>
        </button>

        <div className="h-px bg-gray-700 my-2" />

        <button onClick={() => handleAction('switch_plasma')} className="flex items-center gap-4 p-4 rounded bg-gray-800 hover:bg-steam-blue hover:text-white transition-colors group">
        <MonitorX className="text-purple-400 group-hover:text-white" />
        <span className="font-bold text-lg">{t.system.switch_plasma}</span>
        </button>
        </div>
        </div>
        </div>
    );
};
