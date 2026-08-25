package dev.rustdroid.examples.gradle;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

public final class MainActivity extends Activity {
    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        TextView view = new TextView(this);
        view.setText("RustDroid Gradle fixture launched");
        view.setTextSize(20);
        view.setPadding(48, 48, 48, 48);
        setContentView(view);
    }
}
