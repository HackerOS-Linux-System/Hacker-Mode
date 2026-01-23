import React from 'react';

interface ToggleProps {
    label: string;
    checked: boolean;
    onChange: (checked: boolean) => void;
}

export const Toggle: React.FC<ToggleProps> = ({ label, checked, onChange }) => {
    return (
        <div
        className="flex items-center justify-between p-4 bg-[#1a1c22] border border-gray-800 rounded-sm hover:bg-[#23262e] cursor-pointer transition-colors group"
        onClick={() => onChange(!checked)}
        >
        <span className="text-steam-text font-medium group-hover:text-white transition-colors">{label}</span>
        <div className={`w-14 h-7 flex items-center rounded-full p-1 transition-colors duration-300 ${checked ? 'bg-steam-blue' : 'bg-gray-700'}`}>
        <div
        className={`bg-white w-5 h-5 rounded-full shadow-md transform transition-transform duration-300 ${checked ? 'translate-x-7' : 'translate-x-0'}`}
        />
        </div>
        </div>
    );
};
