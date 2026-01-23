import React, { useState } from 'react';
import { AppConfig } from '../types';
import { systemService } from '../services/systemService';
import { Gamepad2, AlertCircle } from 'lucide-react';

interface LauncherProps {
    apps: AppConfig[];
}

// Sub-component to handle individual image error states cleanly
const GameCard: React.FC<{ app: AppConfig; onLaunch: (id: string) => void }> = ({ app, onLaunch }) => {
    const [imgError, setImgError] = useState(false);

    return (
        <button
        onClick={() => onLaunch(app.id)}
        className="group relative aspect-[3/4] rounded-lg overflow-hidden transition-all duration-300 focus:outline-none focus:ring-4 focus:ring-steam-highlight hover:scale-[1.05] hover:shadow-[0_0_40px_rgba(26,159,255,0.3)] bg-[#1e2329]"
        >
        {/* Background Image/Gradient */}
        <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/20 to-transparent z-10" />

        {/* Content Layer */}
        <div className="absolute inset-0 flex items-center justify-center p-8 z-0 bg-[#15191e]">
        {!imgError ? (
            <img
            src={app.iconUrl}
            alt={app.name}
            className="w-full h-full object-contain drop-shadow-2xl transition-transform duration-500 group-hover:scale-110"
            onError={() => setImgError(true)}
            />
        ) : (
            <div className="flex flex-col items-center justify-center text-gray-600 group-hover:text-steam-blue transition-colors gap-4">
            <Gamepad2 size={64} strokeWidth={1} />
            <span className="text-xs uppercase tracking-widest opacity-50">No Image</span>
            </div>
        )}
        </div>

        {/* Label - Always visible for clarity, especially if image is missing */}
        <div className="absolute bottom-0 left-0 right-0 p-6 z-20 flex flex-col justify-end">
        <span className={`block text-xl font-bold tracking-wide transition-colors ${imgError ? 'text-gray-200' : 'text-white group-hover:text-steam-highlight'}`}>
        {app.name}
        </span>
        <span className="text-xs text-steam-blue uppercase tracking-widest opacity-0 group-hover:opacity-100 transition-opacity delay-75 mt-1 font-bold">
        Click to Launch
        </span>
        </div>

        {/* Selection Border (Steam Deck Style) */}
        <div className="absolute inset-0 border-4 border-transparent group-hover:border-steam-highlight/50 rounded-lg pointer-events-none transition-colors" />
        </button>
    );
};

export const Launcher: React.FC<LauncherProps> = ({ apps }) => {
    const handleLaunch = (appId: string) => {
        systemService.launchApp(appId);
    };

    return (
        <div className="w-full h-full flex flex-col justify-center px-16 pt-16 overflow-y-auto">
        <div className="flex items-center gap-4 mb-8">
        <h2 className="text-2xl font-light text-gray-400 tracking-wider">LIBRARY</h2>
        <div className="h-px flex-1 bg-gradient-to-r from-gray-800 to-transparent"></div>
        <span className="text-sm font-mono text-gray-600">{apps.length} GAMES</span>
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-8 pb-20">
        {apps.map((app) => (
            <GameCard key={app.id} app={app} onLaunch={handleLaunch} />
        ))}
        </div>
        </div>
    );
};
