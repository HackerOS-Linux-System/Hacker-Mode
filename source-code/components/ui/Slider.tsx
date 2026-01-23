import React from 'react';
import * as Lucide from 'lucide-react';

interface SliderProps {
    label: string;
    value: number;
    min?: number;
    max?: number;
    onChange: (val: number) => void;
    icon?: keyof typeof Lucide;
}

export const Slider: React.FC<SliderProps> = ({ label, value, min = 0, max = 100, onChange, icon }) => {
    const Icon = icon ? Lucide[icon] as React.ElementType : null;
    const percentage = ((value - min) / (max - min)) * 100;

    return (
        <div className="flex flex-col gap-2 p-2 group">
        <div className="flex justify-between text-steam-text text-sm mb-1 group-hover:text-white">
        <span className="flex items-center gap-2">
        {Icon && <Icon size={16} />}
        {label}
        </span>
        <span className="font-mono">{value}%</span>
        </div>
        <div className="relative w-full h-8 flex items-center cursor-pointer" onClick={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const newValue = Math.round((x / rect.width) * (max - min) + min);
            onChange(Math.max(min, Math.min(max, newValue)));
        }}>
        {/* Track Background */}
        <div className="absolute w-full h-2 bg-gray-700 rounded-full overflow-hidden">
        {/* Active Track */}
        <div
        className="h-full bg-steam-blue transition-all duration-150 ease-out"
        style={{ width: `${percentage}%` }}
        />
        </div>

        {/* Thumb */}
        <div
        className="absolute h-5 w-5 bg-white rounded-full shadow-lg transition-all duration-150 ease-out"
        style={{ left: `calc(${percentage}% - 10px)` }}
        />
        </div>
        </div>
    );
};
