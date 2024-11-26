// Calculator.java
package com.example;

import com.example.DataHolder;

public class Calculator {
    private int lastResult;

    public Calculator() {
        this.lastResult = 0;
    }

    public int add(int a, int b) {
        lastResult = a + b;
        return lastResult;
    }

    public int multiply(int a, int b) {
        lastResult = a * b;
        return lastResult;
    }

    public int getLastResult() {
        return lastResult;
    }

    public static String getVersion() {
        return "Calculator v1.0";
    }

    // Example of working with strings
    public String formatResult(String prefix) {
        return prefix + ": " + lastResult;
    }

    public int dataHolderTest() {
     DataHolder holder = new DataHolder();
      return holder.test();
  }
}
